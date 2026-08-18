use clap::{Args as ClapArgs, ValueEnum};
use serde::Deserialize;

#[derive(ClapArgs, Deserialize, Clone, Debug, Default)]
pub struct Args {
    /// Project directory / name
    pub name: Option<String>,

    #[arg(long, value_enum, default_value = "npm")]
    #[serde(default)]
    pub package_manager: PackageManager,

    /// ORM used to talk to Postgres from the generated DatabaseModule
    #[arg(long, value_enum, default_value = "drizzle")]
    #[serde(default)]
    pub orm: DbOrm,

    /// Write files without running the package manager install
    #[arg(long)]
    #[serde(default)]
    pub skip_install: bool,

    /// Don't run `git init` in the new project
    #[arg(long)]
    #[serde(default)]
    pub skip_git: bool,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum, Deserialize)]
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

#[derive(Clone, Copy, Debug, Default, ValueEnum, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DbOrm {
    #[default]
    Drizzle,
    Typeorm,
    Prisma,
}

impl DbOrm {
    pub fn as_str(self) -> &'static str {
        match self {
            DbOrm::Drizzle => "drizzle",
            DbOrm::Typeorm => "typeorm",
            DbOrm::Prisma => "prisma",
        }
    }
}
