mod args;
mod listeners;

use anyhow::Result;
use self_update::backends::github::Update;
use self_update::cargo_crate_version;

pub use args::Args;

use crate::shared::context::Context;

const REPO_OWNER: &str = "ateeq1999";
const REPO_NAME: &str = "ci";
const BIN_NAME: &str = "ci";

pub fn run(args: &Args, ctx: &Context) -> Result<()> {
    listeners::bus(ctx).run("update", |events| {
        events.updated("Checking for a newer release");
        let status = Update::configure()
            .repo_owner(REPO_OWNER)
            .repo_name(REPO_NAME)
            .bin_name(BIN_NAME)
            .show_download_progress(true)
            .no_confirm(args.yes)
            .current_version(cargo_crate_version!())
            .build()?
            .update()?;

        // self_update's own status line above doesn't end with a newline
        // (it's designed to be followed by its own inline result) — start
        // fresh so ours doesn't get appended to it.
        println!();

        Ok(if status.updated() {
            format!("Updated to {}", status.version())
        } else {
            format!("Already running the latest version ({})", status.version())
        })
    })
}
