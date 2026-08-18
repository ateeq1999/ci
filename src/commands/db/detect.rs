use std::path::Path;

use anyhow::{bail, Context as _, Result};
use serde::Deserialize;

use crate::shared::context::Context;
use crate::shared::db_orm::{DbOrm, DrizzleDriver};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectConfig {
    orm: DbOrm,
    #[serde(default)]
    driver: DrizzleDriver,
}

#[derive(Debug)]
pub struct Detected {
    pub orm: DbOrm,
    pub driver: DrizzleDriver,
}

/// Reads `<root>/ci/config.json` for the ORM/driver a project was
/// scaffolded with (see `db.md`). Falls back to a marker-file scan for
/// projects created before that file existed. `orm_override` (from
/// `ci db`'s `--orm` flag) skips detection entirely.
pub fn detect(ctx: &Context, root: &Path, orm_override: Option<DbOrm>) -> Result<Detected> {
    if let Some(orm) = orm_override {
        return Ok(Detected {
            orm,
            driver: DrizzleDriver::default(),
        });
    }

    let config_path = root.join("ci/config.json");
    if let Some(raw) = ctx.fs.try_read_to_string(&config_path)? {
        let config: ProjectConfig = serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse {}", config_path.display()))?;
        return Ok(Detected {
            orm: config.orm,
            driver: config.driver,
        });
    }

    let markers = [
        (root.join("drizzle.config.ts"), DbOrm::Drizzle),
        (root.join("prisma/schema.prisma"), DbOrm::Prisma),
        (root.join("src/database/data-source.ts"), DbOrm::Typeorm),
    ];
    let mut found = Vec::new();
    for (path, orm) in &markers {
        if ctx.fs.try_read_to_string(path)?.is_some() {
            found.push(*orm);
        }
    }

    match found.as_slice() {
        [orm] => Ok(Detected {
            orm: *orm,
            driver: DrizzleDriver::default(),
        }),
        [] => bail!(
            "couldn't detect an ORM in {} — no ci/config.json and no marker file \
             (drizzle.config.ts, prisma/schema.prisma, src/database/data-source.ts) found. \
             Pass --orm explicitly.",
            root.display()
        ),
        _ => bail!(
            "ambiguous ORM in {} — found markers for more than one ORM. Pass --orm explicitly.",
            root.display()
        ),
    }
}

#[cfg(test)]
mod tests;
