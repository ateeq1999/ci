mod args;
mod commands;
mod fs;
mod json_payload;
mod templates;

use args::Cli;
use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    commands::run(&cli)
}
