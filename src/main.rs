mod args;
mod commands;

use args::Cli;
use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    commands::run(&cli)
}
