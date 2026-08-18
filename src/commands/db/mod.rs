mod args;
mod detect;

use std::path::Path;

use anyhow::{Result, bail};

pub use args::Args;
use args::{Command, MigrateAction};

use crate::shared::context::Context;
use crate::shared::db_orm::{DbOrm, DrizzleDriver};
use crate::shared::ui;

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

pub fn run(args: &Args, ctx: &Context) -> Result<()> {
    let root = std::env::current_dir()?;
    let is_production = std::env::var("NODE_ENV").as_deref() == Ok("production");

    match &args.command {
        Command::Init(run_args) => {
            let detected = detect::detect(ctx, &root, run_args.orm)?;
            ui::step(&format!(
                "Setting up the database ({})",
                detected.orm.as_str()
            ));
            init(ctx, &root, detected.orm)?;
            ui::success("Database initialized");
            Ok(())
        }
        Command::Migrate(migrate_args) => {
            let detected = detect::detect(ctx, &root, migrate_args.orm)?;
            match &migrate_args.action {
                None => {
                    ui::step(&format!(
                        "Applying pending migrations ({})",
                        detected.orm.as_str()
                    ));
                    migrate(ctx, &root, detected.orm)?;
                    ui::success("Migrations applied");
                    Ok(())
                }
                Some(MigrateAction::Fresh(a)) => {
                    guard_destructive(
                        "drop all tables and re-run every migration",
                        a.yes,
                        a.force,
                        is_production,
                    )?;
                    ui::step(&format!(
                        "Dropping and rebuilding the database ({})",
                        detected.orm.as_str()
                    ));
                    migrate_fresh(ctx, &root, detected.orm, detected.driver)?;
                    ui::success("Database is fresh");
                    Ok(())
                }
                Some(MigrateAction::Refresh(a)) => {
                    guard_destructive(
                        "roll back and re-apply every migration",
                        a.yes,
                        a.force,
                        is_production,
                    )?;
                    ui::step(&format!(
                        "Refreshing the database ({})",
                        detected.orm.as_str()
                    ));
                    migrate_refresh(ctx, &root, detected.orm, detected.driver)?;
                    ui::success("Database refreshed");
                    Ok(())
                }
                Some(MigrateAction::Rollback(a)) => {
                    guard_destructive(
                        &format!("roll back the last {} migration(s)", a.step),
                        a.yes,
                        a.force,
                        is_production,
                    )?;
                    ui::step(&format!(
                        "Rolling back {} migration(s) ({})",
                        a.step,
                        detected.orm.as_str()
                    ));
                    migrate_rollback(ctx, &root, detected.orm, a.step)?;
                    ui::success(&format!("Rolled back {} migration(s)", a.step));
                    Ok(())
                }
            }
        }
        Command::Seed(run_args) => {
            detect::detect(ctx, &root, run_args.orm)?;
            bail!(
                "`ci db seed` isn't implemented yet — no seed script template exists for \
                 any ORM yet. See db.md's Gap 2/3."
            )
        }
    }
}

fn init(ctx: &Context, root: &Path, orm: DbOrm) -> Result<()> {
    match orm {
        DbOrm::Drizzle => {
            ui::step("Generating the initial migration");
            ctx.commands
                .run("npx", &["drizzle-kit", "generate"], root)?;
            ui::step("Applying migrations");
            ctx.commands.run("npx", &["drizzle-kit", "migrate"], root)
        }
        DbOrm::Prisma => {
            ui::step("Creating and applying the initial migration");
            ctx.commands
                .run("npx", &["prisma", "migrate", "dev", "--name", "init"], root)
        }
        DbOrm::Typeorm => {
            ui::step("Generating the initial migration");
            ctx.commands.run(
                "npx",
                &[
                    "typeorm-ts-node-commonjs",
                    "migration:generate",
                    "src/database/migrations/init",
                    "-d",
                    "src/database/data-source.ts",
                ],
                root,
            )?;
            ui::step("Applying migrations");
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

fn migrate(ctx: &Context, root: &Path, orm: DbOrm) -> Result<()> {
    match orm {
        DbOrm::Drizzle => ctx.commands.run("npx", &["drizzle-kit", "migrate"], root),
        DbOrm::Prisma => ctx
            .commands
            .run("npx", &["prisma", "migrate", "deploy"], root),
        DbOrm::Typeorm => ctx.commands.run(
            "npx",
            &[
                "typeorm-ts-node-commonjs",
                "migration:run",
                "-d",
                "src/database/data-source.ts",
            ],
            root,
        ),
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
    bail!("DATABASE_URL not set in {}", env_path.display());
}

/// Drops and recreates the app's schema for Drizzle — see `DROP_SCHEMA_SQL`
/// for why both `public` and `drizzle` get dropped. There's no drizzle-kit
/// command for this (confirmed against its docs — `generate`/`migrate`/
/// `push`/`pull`/`studio`/`check`/`up`/`export`, nothing that touches
/// existing tables destructively), so this connects directly with
/// whichever driver the project is configured for.
fn drop_drizzle_schema(ctx: &Context, root: &Path, driver: DrizzleDriver) -> Result<()> {
    ui::step("Dropping existing tables and migration history");
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

fn migrate_fresh(ctx: &Context, root: &Path, orm: DbOrm, driver: DrizzleDriver) -> Result<()> {
    match orm {
        DbOrm::Drizzle => {
            drop_drizzle_schema(ctx, root, driver)?;
            ui::step("Regenerating the migration");
            ctx.commands
                .run("npx", &["drizzle-kit", "generate"], root)?;
            ui::step("Applying migrations");
            ctx.commands.run("npx", &["drizzle-kit", "migrate"], root)
        }
        DbOrm::Prisma => {
            ui::step("Resetting the database (drop, re-apply, re-seed)");
            ctx.commands
                .run("npx", &["prisma", "migrate", "reset", "--force"], root)
        }
        DbOrm::Typeorm => {
            ui::step("Dropping the schema");
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
            ui::step("Applying migrations");
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

/// For Prisma, identical to `migrate_fresh` — `prisma migrate reset` already
/// drops, re-applies, and re-seeds in one command, so it satisfies both
/// `fresh` and `refresh` semantics with nothing extra to implement. Same
/// story for Drizzle: with no down-migration story at all, "roll back
/// everything then re-apply" and "drop everything then re-apply" are the
/// same operation, so it reuses `migrate_fresh`'s drop-and-rebuild.
fn migrate_refresh(ctx: &Context, root: &Path, orm: DbOrm, driver: DrizzleDriver) -> Result<()> {
    match orm {
        DbOrm::Prisma => {
            ui::step("Resetting the database (drop, re-apply, re-seed)");
            ctx.commands
                .run("npx", &["prisma", "migrate", "reset", "--force"], root)
        }
        DbOrm::Drizzle => migrate_fresh(ctx, root, orm, driver),
        DbOrm::Typeorm => bail!(
            "`ci db migrate refresh` isn't implemented for TypeORM yet — safely reverting \
             *all* migrations needs to know how many exist first; only \
             `migrate rollback --step N` (explicit count) is implemented so far."
        ),
    }
}

fn migrate_rollback(ctx: &Context, root: &Path, orm: DbOrm, step: u32) -> Result<()> {
    match orm {
        DbOrm::Typeorm => {
            for i in 1..=step {
                if step > 1 {
                    ui::step(&format!("Reverting migration {i}/{step}"));
                }
                ctx.commands.run(
                    "npx",
                    &[
                        "typeorm-ts-node-commonjs",
                        "migration:revert",
                        "-d",
                        "src/database/data-source.ts",
                    ],
                    root,
                )?;
            }
            Ok(())
        }
        DbOrm::Drizzle => bail!(
            "`ci db migrate rollback` isn't supported for Drizzle — drizzle-kit has no \
             rollback command. Hand-write a down migration or restore from a backup."
        ),
        DbOrm::Prisma => bail!(
            "`ci db migrate rollback` isn't supported for Prisma — it has no \
             single-migration undo, only `ci db migrate fresh` (prisma migrate reset)."
        ),
    }
}

fn guard_destructive(action: &str, yes: bool, force: bool, is_production: bool) -> Result<()> {
    if is_production && !force {
        bail!("refusing to run `{action}` with NODE_ENV=production without --force");
    }
    if !yes {
        ui::warn(&format!("This will {action}."));
        if !confirm("Continue?")? {
            bail!("aborted");
        }
    }
    Ok(())
}

fn confirm(prompt: &str) -> Result<bool> {
    use std::io::Write as _;
    print!("{prompt} [y/N] ");
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}

#[cfg(test)]
mod tests;
