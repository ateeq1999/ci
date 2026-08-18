use clap::{ArgAction, Parser, Subcommand};

use crate::commands::{init, update};

#[derive(Parser)]
#[command(name = "ci", author, version, about, long_about = None, disable_version_flag = true)]
pub struct Cli {
    /// Provide this command's arguments as a JSON payload instead of flags.
    /// Accepts a literal JSON string, or `@path/to/file.json` to read one.
    #[arg(short = 'j', long, global = true)]
    pub json: Option<String>,

    /// Print version
    #[arg(short = 'v', long = "version", action = ArgAction::Version)]
    version: (),

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new NestJS project
    Init(init::Args),
    /// Update this CLI to the latest release
    Update(update::Args),
}
