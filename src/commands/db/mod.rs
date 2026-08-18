mod args;
mod detect;

use std::path::Path;

use anyhow::{bail, Result};

pub use args::Args;
use args::{Command, MigrateAction};

use crate::context::Context;
use crate::db_orm::DbOrm;

pub fn run(args: &Args, ctx: &Context) -> Result<()> {
    let root = std::env::current_dir()?;
    let is_production = std::env::var("NODE_ENV").as_deref() == Ok("production");

    match &args.command {
        Command::Init(run_args) => {
            let detected = detect::detect(ctx, &root, run_args.orm)?;
            init(ctx, &root, detected.orm)
        }
        Command::Migrate(migrate_args) => {
            let detected = detect::detect(ctx, &root, migrate_args.orm)?;
            match &migrate_args.action {
                None => migrate(ctx, &root, detected.orm),
                Some(MigrateAction::Fresh(a)) => {
                    guard_destructive(
                        "drop all tables and re-run every migration",
                        a.yes,
                        a.force,
                        is_production,
                    )?;
                    migrate_fresh(ctx, &root, detected.orm)
                }
                Some(MigrateAction::Refresh(a)) => {
                    guard_destructive(
                        "roll back and re-apply every migration",
                        a.yes,
                        a.force,
                        is_production,
                    )?;
                    migrate_refresh(ctx, &root, detected.orm)
                }
                Some(MigrateAction::Rollback(a)) => {
                    guard_destructive(
                        &format!("roll back the last {} migration(s)", a.step),
                        a.yes,
                        a.force,
                        is_production,
                    )?;
                    migrate_rollback(ctx, &root, detected.orm, a.step)
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
            ctx.commands.run("npx", &["drizzle-kit", "generate"], root)?;
            ctx.commands.run("npx", &["drizzle-kit", "migrate"], root)
        }
        DbOrm::Prisma => ctx
            .commands
            .run("npx", &["prisma", "migrate", "dev", "--name", "init"], root),
        DbOrm::Typeorm => {
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

fn migrate_fresh(ctx: &Context, root: &Path, orm: DbOrm) -> Result<()> {
    match orm {
        DbOrm::Drizzle => bail!(
            "`ci db migrate fresh` isn't supported for Drizzle yet — drizzle-kit has no \
             schema-drop command. See db.md's Gap 4."
        ),
        DbOrm::Prisma => ctx
            .commands
            .run("npx", &["prisma", "migrate", "reset", "--force"], root),
        DbOrm::Typeorm => {
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
/// `fresh` and `refresh` semantics with nothing extra to implement.
fn migrate_refresh(ctx: &Context, root: &Path, orm: DbOrm) -> Result<()> {
    match orm {
        DbOrm::Prisma => ctx
            .commands
            .run("npx", &["prisma", "migrate", "reset", "--force"], root),
        DbOrm::Drizzle => bail!(
            "`ci db migrate refresh` isn't supported for Drizzle yet — drizzle-kit has no \
             down-migration story. See db.md's Gap 4."
        ),
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
            for _ in 0..step {
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
    if !yes && !confirm(&format!("This will {action}. Continue?"))? {
        bail!("aborted");
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
