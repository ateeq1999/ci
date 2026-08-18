use clap::{Parser, Subcommand};

use crate::commands::{init, update};

#[derive(Parser)]
#[command(name = "ci", author, version, about, long_about = None)]
pub struct Cli {
    /// Provide this command's arguments as a JSON payload instead of flags.
    /// Accepts a literal JSON string, or `@path/to/file.json` to read one.
    #[arg(long, global = true)]
    pub json: Option<String>,

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
