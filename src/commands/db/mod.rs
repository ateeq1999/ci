//! `ci db` — thin dispatch only. Every actual subcommand
//! (`init`/`migrate`/`migrate fresh`/`migrate refresh`/`migrate
//! rollback`/`seed`) is its own self-contained module, each owning its
//! own logic and tests. `detect`/`listeners`/`support` are shared
//! infrastructure, not subcommands themselves.

mod args;
mod detect;
mod init;
mod listeners;
mod migrate;
mod seed;
mod support;

pub use args::Args;
use args::Command;

use crate::shared::context::Context;

pub fn run(args: &Args, ctx: &Context) -> anyhow::Result<()> {
    let root = std::env::current_dir()?;
    let bus = listeners::bus(ctx, &root);

    match &args.command {
        Command::Init(run_args) => init::run(ctx, &root, &bus, run_args),
        Command::Migrate(migrate_args) => migrate::run(ctx, &root, &bus, migrate_args),
        Command::Seed(run_args) => seed::run(ctx, &root, &bus, run_args),
    }
}
