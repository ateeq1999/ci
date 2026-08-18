mod args;
mod commands;
mod context;
mod db_orm;
mod fs;
mod json_payload;

use args::Cli;
use clap::Parser;

fn main() -> anyhow::Result<()> {
    if args::wants_help_all() {
        args::print_full_help();
        return Ok(());
    }

    let cli = Cli::parse();

    commands::run(&cli)
}
