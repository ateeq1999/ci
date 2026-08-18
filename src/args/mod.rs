use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Deserialize;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Provide this command's arguments as a JSON payload instead of flags.
    /// Accepts a literal JSON string, or `@path/to/file.json` to read one.
    #[arg(long, global = true)]
    pub json: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new NestJS project
    Init(InitArgs),
}

#[derive(Args, Deserialize, Clone, Debug, Default)]
pub struct InitArgs {
    /// Project directory / name
    pub name: Option<String>,

    #[arg(long, value_enum, default_value = "npm")]
    #[serde(default)]
    pub package_manager: PackageManager,

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
