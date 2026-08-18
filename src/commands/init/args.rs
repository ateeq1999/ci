use clap::{Args as ClapArgs, ValueEnum};
use serde::{Deserialize, Serialize};

pub use crate::db_orm::{DbOrm, DrizzleDriver};

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

#[derive(Clone, Copy, Debug, Default, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    #[default]
    Npm,
    Pnpm,
    Yarn,
}

impl PackageManager {
    pub fn command(self) -> &'static str {
        match self {
            PackageManager::Npm => "npm",
            PackageManager::Pnpm => "pnpm",
            PackageManager::Yarn => "yarn",
        }
    }
}
