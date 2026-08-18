//! Builds `ci/config.json` — the record `ci db` (see `db.md`) reads back to
//! know which ORM/driver/package manager a project was scaffolded with,
//! instead of re-detecting it from marker files every time.

use anyhow::Result;
use serde::Serialize;

use super::args::{DbOrm, DrizzleDriver, PackageManager};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectConfig {
    ci_version: &'static str,
    orm: DbOrm,
    #[serde(skip_serializing_if = "Option::is_none")]
    driver: Option<DrizzleDriver>,
    package_manager: PackageManager,
}

pub fn render(orm: DbOrm, driver: DrizzleDriver, package_manager: PackageManager) -> Result<String> {
    let config = ProjectConfig {
        ci_version: env!("CARGO_PKG_VERSION"),
        orm,
        driver: (orm == DbOrm::Drizzle).then_some(driver),
        package_manager,
    };
    Ok(serde_json::to_string_pretty(&config)? + "\n")
}
