mod args;
mod commands;
mod shared;

use args::Cli;
use clap::Parser;
use shared::context::Context;

fn main() {
    if args::wants_help_all() {
        args::print_full_help();
        return;
    }

    let cli = Cli::parse();
    let ctx = Context::real();

    if commands::run(&cli, &ctx).is_err() {
        std::process::exit(1);
    }
}
