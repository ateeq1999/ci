use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
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

    /// The verb for adding *specific new* packages — not the same as
    /// `install` everywhere: npm uses `install` for this too, but pnpm and
    /// yarn reserve `install` for "install from the lockfile only, don't
    /// add anything new" and use `add` for new dependencies.
    pub fn add_verb(self) -> &'static str {
        match self {
            PackageManager::Npm => "install",
            PackageManager::Pnpm | PackageManager::Yarn => "add",
        }
    }
}
