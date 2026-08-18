use clap::{Args as ClapArgs, Subcommand};

use crate::db_orm::DbOrm;

#[derive(ClapArgs, Clone, Debug)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    /// First-time setup: create and apply the initial migration
    Init(RunArgs),
    /// Apply pending migrations, or run a migrate subcommand
    Migrate(MigrateArgs),
    /// Run the project's seed script
    Seed(RunArgs),
}

#[derive(ClapArgs, Clone, Debug, Default)]
pub struct RunArgs {
    /// Override ORM detection (reads `ci/config.json` by default)
    #[arg(long, value_enum)]
    pub orm: Option<DbOrm>,
}

#[derive(ClapArgs, Clone, Debug)]
pub struct MigrateArgs {
    #[command(subcommand)]
    pub action: Option<MigrateAction>,

    /// Override ORM detection (reads `ci/config.json` by default)
    #[arg(long, value_enum, global = true)]
    pub orm: Option<DbOrm>,
}

#[derive(Subcommand, Clone, Debug)]
pub enum MigrateAction {
    /// Drop everything and re-run all migrations from zero
    Fresh(DestructiveArgs),
    /// Roll back all migrations, then re-apply them
    Refresh(DestructiveArgs),
    /// Undo the last migration (or --step N)
    Rollback(RollbackArgs),
}

#[derive(ClapArgs, Clone, Debug, Default)]
pub struct DestructiveArgs {
    /// Skip the confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Required alongside --yes when NODE_ENV=production
    #[arg(short = 'f', long)]
    pub force: bool,
}

#[derive(ClapArgs, Clone, Debug)]
pub struct RollbackArgs {
    /// Skip the confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Required alongside --yes when NODE_ENV=production
    #[arg(short = 'f', long)]
    pub force: bool,

    /// Number of migrations to roll back
    #[arg(long, default_value_t = 1)]
    pub step: u32,
}
