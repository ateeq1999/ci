//! `ci add` — thin dispatch only, same shape as `db`. Every subcommand
//! (`validation`/`cache`/`schedule`/`queue`/`logger`) is its own
//! self-contained module.

mod args;
mod cache;
mod listeners;
mod logger;
mod patch;
mod queue;
mod schedule;
mod validation;

pub use args::Args;
use args::Command;

use crate::shared::context::Context;

pub fn run(args: &Args, ctx: &Context) -> anyhow::Result<()> {
    let root = std::env::current_dir()?;
    let bus = listeners::bus(ctx, &root);

    match &args.command {
        Command::Validation => validation::run(ctx, &root, &bus),
        Command::Cache => cache::run(ctx, &root, &bus),
        Command::Schedule => schedule::run(ctx, &root, &bus),
        Command::Queue => queue::run(ctx, &root, &bus),
        Command::Logger => logger::run(ctx, &root, &bus),
    }
}
