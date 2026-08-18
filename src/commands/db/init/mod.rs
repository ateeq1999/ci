//! `ci db init` — self-contained: its own dispatch, its own per-ORM
//! command bodies, its own tests.

use std::path::Path;

use anyhow::Result;

use crate::shared::context::Context;
use crate::shared::db_orm::DbOrm;
use crate::shared::events::{EventBus, Updates};

use super::args::RunArgs;
use super::detect;

pub fn run(ctx: &Context, root: &Path, bus: &EventBus, args: &RunArgs) -> Result<()> {
    bus.run("db init", |events| {
        let detected = detect::detect(ctx, root, args.orm)?;
        events.updated(format!(
            "Setting up the database ({})",
            detected.orm.as_str()
        ));
        run_for_orm(ctx, root, detected.orm, events)?;
        Ok("Database initialized".to_string())
    })
}

fn run_for_orm(ctx: &Context, root: &Path, orm: DbOrm, events: &Updates) -> Result<()> {
    match orm {
        DbOrm::Drizzle => {
            events.updated("Generating the initial migration");
            ctx.commands
                .run("npx", &["drizzle-kit", "generate"], root)?;
            events.updated("Applying migrations");
            ctx.commands.run("npx", &["drizzle-kit", "migrate"], root)
        }
        DbOrm::Prisma => {
            events.updated("Creating and applying the initial migration");
            ctx.commands
                .run("npx", &["prisma", "migrate", "dev", "--name", "init"], root)
        }
        DbOrm::Typeorm => {
            events.updated("Generating the initial migration");
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

#[cfg(test)]
mod tests;
