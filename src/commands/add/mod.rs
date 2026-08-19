//! `ci add` — thin dispatch only, same shape as `db`. Every subcommand
//! (`validation`/`cache`) is its own self-contained module.

mod args;
mod cache;
mod listeners;
mod patch;
mod validation;

pub use args::Args;
use args::Command;

use crate::shared::context::Context;

pub fn run(args: &Args, ctx: &Context) -> anyhow::Result<()> {
    let root = std::env::current_dir()?;
    let bus = listeners::bus(ctx);

    match &args.command {
        Command::Validation => validation::run(ctx, &root, &bus),
        Command::Cache => cache::run(ctx, &root, &bus),
    }
}
