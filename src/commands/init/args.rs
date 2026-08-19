use clap::Args as ClapArgs;
use serde::Deserialize;

pub use crate::shared::db_orm::{DbOrm, DrizzleDriver};
pub use crate::shared::package_manager::PackageManager;

#[derive(ClapArgs, Deserialize, Clone, Debug, Default)]
pub struct Args {
    /// Project directory / name
    pub name: Option<String>,

    /// Package manager to run `install` with
    #[arg(long, value_enum, default_value = "npm")]
    #[serde(default)]
    pub package_manager: PackageManager,

    /// ORM used to talk to Postgres from the generated DatabaseModule
    #[arg(long, value_enum, default_value = "drizzle")]
    #[serde(default)]
    pub orm: DbOrm,

    /// Postgres driver Drizzle uses (ignored for other ORMs)
    #[arg(long, value_enum, default_value = "pg")]
    #[serde(default)]
    pub driver: DrizzleDriver,

    /// Write files without running the package manager install
    #[arg(long)]
    #[serde(default)]
    pub skip_install: bool,

    /// Don't run `git init` in the new project
    #[arg(long)]
    #[serde(default)]
    pub skip_git: bool,
}
