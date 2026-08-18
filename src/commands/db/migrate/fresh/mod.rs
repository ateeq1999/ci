//! `ci db migrate fresh` — self-contained: its own drop scripts, its own
//! per-ORM bodies, its own tests. `refresh.rs` calls back into
//! `run_for_orm` for its Drizzle case (see that file for why).

use std::path::Path;

use anyhow::Result;

use crate::shared::context::Context;
use crate::shared::db_orm::{DbOrm, DrizzleDriver};
use crate::shared::events::{EventBus, Updates};

use super::super::args::DestructiveArgs;
use super::super::detect;
use super::super::support::guard_destructive;

/// Drops and recreates `public` (every app table), plus drops the
/// `drizzle` schema drizzle-kit's `migrate` command keeps its
/// `__drizzle_migrations` journal in (default location, per drizzle-kit's
/// docs — not configured otherwise by `drizzle.config.ts`). Dropping only
/// `public` and leaving the journal behind would make the next `migrate`
/// think everything's already applied against a now-empty database.
const DROP_SCHEMA_SQL: &str = "DROP SCHEMA IF EXISTS public CASCADE; CREATE SCHEMA public; DROP SCHEMA IF EXISTS drizzle CASCADE;";

/// Connects with whichever driver the project uses and runs
/// `DROP_SCHEMA_SQL`. The connection string is passed as a `node` argument
/// (after `--`), not interpolated into the script text, so it doesn't need
/// escaping even if it contains quotes.
const PG_DROP_SCRIPT: &str = r#"
const { Client } = require("pg");
(async () => {
  const client = new Client({ connectionString: process.argv[1] });
  await client.connect();
  await client.query(process.argv[2]);
  await client.end();
})().catch((err) => {
  console.error(err);
  process.exit(1);
});
"#;

const POSTGRES_JS_DROP_SCRIPT: &str = r#"
const postgres = require("postgres");
(async () => {
  const sql = postgres(process.argv[1]);
  await sql.unsafe(process.argv[2]);
  await sql.end();
})().catch((err) => {
  console.error(err);
  process.exit(1);
});
"#;

pub fn run(
    ctx: &Context,
    root: &Path,
    bus: &EventBus,
    orm_override: Option<DbOrm>,
    args: &DestructiveArgs,
) -> Result<()> {
    let is_production = std::env::var("NODE_ENV").as_deref() == Ok("production");
    bus.run("db migrate fresh", |events| {
        guard_destructive(
            events,
            "drop all tables and re-run every migration",
            args.yes,
            args.force,
            is_production,
        )?;
        let detected = detect::detect(ctx, root, orm_override)?;
        events.updated(format!(
            "Dropping and rebuilding the database ({})",
            detected.orm.as_str()
        ));
        run_for_orm(ctx, root, detected.orm, detected.driver, events)?;
        Ok("Database is fresh".to_string())
    })
}

/// `pub(super)` (not private) — `refresh.rs`'s Drizzle case is identical to
/// this (no down-migration story means "roll back everything" and "drop
/// everything" are the same operation), so it calls straight into this
/// instead of duplicating it.
pub(super) fn run_for_orm(
    ctx: &Context,
    root: &Path,
    orm: DbOrm,
    driver: DrizzleDriver,
    events: &Updates,
) -> Result<()> {
    match orm {
        DbOrm::Drizzle => {
            drop_drizzle_schema(ctx, root, driver, events)?;
            events.updated("Regenerating the migration");
            ctx.commands
                .run("npx", &["drizzle-kit", "generate"], root)?;
            events.updated("Applying migrations");
            ctx.commands.run("npx", &["drizzle-kit", "migrate"], root)
        }
        DbOrm::Prisma => {
            events.updated("Resetting the database (drop, re-apply, re-seed)");
            ctx.commands
                .run("npx", &["prisma", "migrate", "reset", "--force"], root)
        }
        DbOrm::Typeorm => {
            events.updated("Dropping the schema");
            ctx.commands.run(
                "npx",
                &[
                    "typeorm-ts-node-commonjs",
                    "schema:drop",
                    "-d",
                    "src/database/data-source.ts",
                ],
                root,
            )?;
            events.updated("Applying migrations");
            ctx.commands.run(
                "npx",
                &[
                    "typeorm-ts-node-commonjs",
                    "migration:run",
                    "-d",
                    "src/database/data-source.ts",
                ],
                root,
            )
        }
    }
}

/// Reads `DATABASE_URL` out of the project's `.env` (not `.env.example` —
/// `ci init` copies one to the other, but only `.env` is meant to hold a
/// real, locally-usable value). A hand-rolled single-key scan rather than a
/// full dotenv parser: this only ever needs the one variable, and `.env` is
/// a file this same tool wrote in the first place.
fn read_database_url(ctx: &Context, root: &Path) -> Result<String> {
    let env_path = root.join(".env");
    let contents = ctx.fs.try_read_to_string(&env_path)?.ok_or_else(|| {
        anyhow::anyhow!(
            "{} not found — copy .env.example to .env and set DATABASE_URL",
            env_path.display()
        )
    })?;

    for line in contents.lines() {
        if let Some(value) = line.trim().strip_prefix("DATABASE_URL=") {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                return Ok(value.to_string());
            }
        }
    }
    anyhow::bail!("DATABASE_URL not set in {}", env_path.display());
}

/// Drops and recreates the app's schema for Drizzle — see `DROP_SCHEMA_SQL`
/// for why both `public` and `drizzle` get dropped. There's no drizzle-kit
/// command for this (confirmed against its docs — `generate`/`migrate`/
/// `push`/`pull`/`studio`/`check`/`up`/`export`, nothing that touches
/// existing tables destructively), so this connects directly with
/// whichever driver the project is configured for.
fn drop_drizzle_schema(
    ctx: &Context,
    root: &Path,
    driver: DrizzleDriver,
    events: &Updates,
) -> Result<()> {
    events.updated("Dropping existing tables and migration history");
    let database_url = read_database_url(ctx, root)?;
    let script = match driver {
        DrizzleDriver::Pg => PG_DROP_SCRIPT,
        DrizzleDriver::PostgresJs => POSTGRES_JS_DROP_SCRIPT,
    };
    ctx.commands.run(
        "node",
        &["-e", script, "--", &database_url, DROP_SCHEMA_SQL],
        root,
    )
}

#[cfg(test)]
mod tests;
