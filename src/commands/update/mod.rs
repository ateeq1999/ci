mod args;

use anyhow::Result;
use self_update::backends::github::Update;
use self_update::cargo_crate_version;

pub use args::Args;

const REPO_OWNER: &str = "ateeq1999";
const REPO_NAME: &str = "ci";
const BIN_NAME: &str = "ci";

pub fn run(args: &Args) -> Result<()> {
    let status = Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .show_download_progress(true)
        .no_confirm(args.yes)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;

    if status.updated() {
        println!("Updated to {}", status.version());
    } else {
        println!("Already running the latest version ({})", status.version());
    }
    Ok(())
}
