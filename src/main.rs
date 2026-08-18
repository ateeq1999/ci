mod args;
mod commands;
mod shared;

use args::Cli;
use clap::Parser;
use shared::ui;

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
