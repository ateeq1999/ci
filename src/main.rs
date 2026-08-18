mod args;
mod commands;
mod context;
mod db_orm;
mod fs;
mod json_payload;
mod ui;

use args::Cli;
use clap::Parser;

fn main() {
    if args::wants_help_all() {
        args::print_full_help();
        return;
    }

    let cli = Cli::parse();

    if let Err(err) = commands::run(&cli) {
        ui::error(&format!("{err:#}"));
        std::process::exit(1);
    }
}
