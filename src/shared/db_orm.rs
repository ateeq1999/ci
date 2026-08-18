//! Shared between `commands::init` (writes `ci/config.json` and renders
//! templates for the chosen ORM/driver) and `commands::db` (reads
//! `ci/config.json` back to know which underlying tool to shell out to).

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DrizzleDriver {
    #[default]
    Pg,
    PostgresJs,
}

impl DrizzleDriver {
    pub fn as_str(self) -> &'static str {
        match self {
            DrizzleDriver::Pg => "pg",
            DrizzleDriver::PostgresJs => "postgres-js",
        }
    }
}
