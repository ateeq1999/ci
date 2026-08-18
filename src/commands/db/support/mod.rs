//! Shared by the destructive `migrate` subcommands (`fresh`/`refresh`/
//! `rollback`) — not a subcommand itself, so it has no `run()`/`listeners`
//! of its own, just the confirmation-gate logic each of them calls first.

use anyhow::{Result, bail};

use crate::shared::events::Updates;

pub fn guard_destructive(
    events: &Updates,
    action: &str,
    yes: bool,
    force: bool,
    is_production: bool,
) -> Result<()> {
    if is_production && !force {
        bail!("refusing to run `{action}` with NODE_ENV=production without --force");
    }
    if !yes {
        events.warned(format!("This will {action}."));
        if !confirm("Continue?")? {
            bail!("aborted");
        }
    }
    Ok(())
}

fn confirm(prompt: &str) -> Result<bool> {
    use std::io::Write as _;
    print!("{prompt} [y/N] ");
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}

#[cfg(test)]
mod tests;
