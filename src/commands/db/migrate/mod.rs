//! `ci db migrate` — dispatches to the bare "apply pending migrations"
//! case (handled right here) or one of the three destructive subcommands,
//! each self-contained in its own file.

mod fresh;
mod refresh;
mod rollback;

use std::path::Path;

use anyhow::Result;

use crate::shared::context::Context;
use crate::shared::db_orm::DbOrm;
use crate::shared::events::EventBus;

use super::args::{MigrateAction, MigrateArgs};
use super::detect;

pub fn run(ctx: &Context, root: &Path, bus: &EventBus, args: &MigrateArgs) -> Result<()> {
    match &args.action {
        None => bus.run("db migrate", |events| {
            let detected = detect::detect(ctx, root, args.orm)?;
            events.updated(format!(
                "Applying pending migrations ({})",
                detected.orm.as_str()
            ));
            run_for_orm(ctx, root, detected.orm)?;
            Ok("Migrations applied".to_string())
        }),
        Some(MigrateAction::Fresh(a)) => fresh::run(ctx, root, bus, args.orm, a),
        Some(MigrateAction::Refresh(a)) => refresh::run(ctx, root, bus, args.orm, a),
        Some(MigrateAction::Rollback(a)) => rollback::run(ctx, root, bus, args.orm, a),
    }
}

fn run_for_orm(ctx: &Context, root: &Path, orm: DbOrm) -> Result<()> {
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

#[cfg(test)]
mod tests;
