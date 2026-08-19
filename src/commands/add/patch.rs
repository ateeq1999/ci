//! Editing files that already exist — new capability neither `init` nor
//! `db` ever needed (both only ever write fresh files or shell out).
//! `package.json` is real JSON, so it's parsed and merged. `main.ts`/
//! `app.module.ts` are edited by anchoring on lines `ci` itself wrote via
//! `init`'s templates — not a general TS parser, since there isn't one
//! worth pulling in for two insertion points.

use std::path::Path;

use anyhow::{Result, anyhow, bail};

use crate::shared::context::Context;

/// Adds each `(name, version)` pair to `package.json`'s `dependencies`
/// object, without overwriting a version already there — idempotent by
/// construction.
pub fn add_dependencies(ctx: &Context, root: &Path, deps: &[(&str, &str)]) -> Result<()> {
    let path = root.join("package.json");
    let raw = ctx.fs.try_read_to_string(&path)?.ok_or_else(|| {
        anyhow!(
            "{} not found — run this inside a `ci init`-created project",
            path.display()
        )
    })?;
    let mut json: serde_json::Value = serde_json::from_str(&raw)?;
    let deps_obj = json["dependencies"]
        .as_object_mut()
        .ok_or_else(|| anyhow!("package.json has no \"dependencies\" object"))?;
    for (name, version) in deps {
        deps_obj
            .entry(name.to_string())
            .or_insert_with(|| (*version).into());
    }
    ctx.fs
        .write_file(&path, &(serde_json::to_string_pretty(&json)? + "\n"))
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
