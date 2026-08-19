//! Editing files that already exist — new capability neither `init` nor
//! `db` ever needed (both only ever write fresh files or shell out).
//! `main.ts`/`app.module.ts` are edited by anchoring on lines `ci` itself
//! wrote via `init`'s templates — not a general TS parser, since there
//! isn't one worth pulling in for two insertion points. `package.json`
//! isn't edited directly at all — see `install_dependencies` below.

use std::path::Path;

use anyhow::{Result, anyhow, bail};
use serde::Deserialize;

use crate::shared::context::Context;
use crate::shared::package_manager::PackageManager;

/// Reads `packageManager` out of `<root>/ci/config.json` (the same file
/// `db::detect` reads `orm`/`driver` from) — falls back to
/// `PackageManager::default()` (npm) if the file is missing or
/// unparseable, per the explicit "if not found, use npm" instruction;
/// this is advisory (which installer to shell out to), not something
/// worth failing a whole `ci add` run over.
pub fn detect_package_manager(ctx: &Context, root: &Path) -> PackageManager {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ProjectConfig {
        #[serde(default)]
        package_manager: PackageManager,
    }

    ctx.fs
        .try_read_to_string(&root.join("ci/config.json"))
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<ProjectConfig>(&raw).ok())
        .map(|config| config.package_manager)
        .unwrap_or_default()
}

/// Installs `packages` with whichever package manager the project is
/// configured for — not a `package.json` edit of our own; the package
/// manager itself resolves real versions and writes them in, which is
/// more correct than this tool guessing a semver range.
pub fn install_dependencies(
    ctx: &Context,
    root: &Path,
    package_manager: PackageManager,
    packages: &[&str],
) -> Result<()> {
    let mut args = vec![package_manager.add_verb()];
    args.extend(packages);
    ctx.commands.run(package_manager.command(), &args, root)
}

/// Inserts `line` right after the first line containing `anchor`, unless
/// `already_present_marker` is already in the file (returns `Ok(false)`
/// then — "already configured," not an error). Errors clearly instead of
/// guessing if `anchor` isn't found at all (e.g. the file was hand-edited
/// away from what `init` generated).
pub fn insert_after(
    ctx: &Context,
    path: &Path,
    anchor: &str,
    already_present_marker: &str,
    line: &str,
) -> Result<bool> {
    let contents = ctx
        .fs
        .try_read_to_string(path)?
        .ok_or_else(|| anyhow!("{} not found", path.display()))?;
    if contents.contains(already_present_marker) {
        return Ok(false);
    }

    let mut out = String::new();
    let mut inserted = false;
    for l in contents.lines() {
        out.push_str(l);
        out.push('\n');
        if !inserted && l.contains(anchor) {
            out.push_str(line);
            out.push('\n');
            inserted = true;
        }
    }
    if !inserted {
        bail!(
            "couldn't find `{anchor}` in {} — has it been hand-edited?",
            path.display()
        );
    }
    ctx.fs.write_file(path, &out)?;
    Ok(true)
}

/// Inserts `lines` right after the *last* line starting with `import `,
/// rather than a fixed anchor — so when more than one `ci add` subcommand
/// patches the same file (e.g. `cache`/`schedule`/`queue` all touching
/// `app.module.ts`), each new import lands after whatever the previous
/// subcommand already inserted, instead of every subcommand colliding on
/// the same fixed anchor line. Same idempotency/error shape as
/// `insert_after`.
pub fn insert_after_last_import(
    ctx: &Context,
    path: &Path,
    already_present_marker: &str,
    lines: &str,
) -> Result<bool> {
    let contents = ctx
        .fs
        .try_read_to_string(path)?
        .ok_or_else(|| anyhow!("{} not found", path.display()))?;
    if contents.contains(already_present_marker) {
        return Ok(false);
    }

    let all_lines: Vec<&str> = contents.lines().collect();
    let last_import = all_lines
        .iter()
        .rposition(|l| l.trim_start().starts_with("import "))
        .ok_or_else(|| anyhow!("no `import` line found in {}", path.display()))?;

    let mut out = String::new();
    for (i, l) in all_lines.iter().enumerate() {
        out.push_str(l);
        out.push('\n');
        if i == last_import {
            out.push_str(lines);
            out.push('\n');
        }
    }
    ctx.fs.write_file(path, &out)?;
    Ok(true)
}

/// Inserts `item` as a new element right after the array's opening `[` on
/// the line containing `array_anchor` (e.g. `"imports: ["`), matching
/// that line's indentation plus one level. Same idempotency/error shape
/// as `insert_after`.
pub fn insert_into_array(
    ctx: &Context,
    path: &Path,
    array_anchor: &str,
    already_present_marker: &str,
    item: &str,
) -> Result<bool> {
    let contents = ctx
        .fs
        .try_read_to_string(path)?
        .ok_or_else(|| anyhow!("{} not found", path.display()))?;
    if contents.contains(already_present_marker) {
        return Ok(false);
    }

    let mut out = String::new();
    let mut inserted = false;
    for l in contents.lines() {
        out.push_str(l);
        out.push('\n');
        if !inserted && l.contains(array_anchor) {
            let indent: String = l.chars().take_while(|c| c.is_whitespace()).collect();
            out.push_str(&indent);
            out.push_str("  ");
            out.push_str(item);
            out.push('\n');
            inserted = true;
        }
    }
    if !inserted {
        bail!(
            "couldn't find `{array_anchor}` in {} — has it been hand-edited?",
            path.display()
        );
    }
    ctx.fs.write_file(path, &out)?;
    Ok(true)
}

/// Appends `line` to the end of a file (e.g. a new `.env`/`.env.example`
/// variable). Same idempotency shape as the others, minus the "anchor not
/// found" error case — there's nothing to anchor on, only something to
/// avoid duplicating.
pub fn append_line(
    ctx: &Context,
    path: &Path,
    already_present_marker: &str,
    line: &str,
) -> Result<bool> {
    let Some(mut contents) = ctx.fs.try_read_to_string(path)? else {
        bail!("{} not found", path.display());
    };
    if contents.contains(already_present_marker) {
        return Ok(false);
    }

    if !contents.is_empty() && !contents.ends_with('\n') {
        contents.push('\n');
    }
    contents.push_str(line);
    contents.push('\n');
    ctx.fs.write_file(path, &contents)?;
    Ok(true)
}

#[cfg(test)]
mod tests;
