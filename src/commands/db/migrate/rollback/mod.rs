//! `ci db migrate rollback` — self-contained.

use std::path::Path;

use anyhow::{Result, bail};

use crate::shared::context::Context;
use crate::shared::db_orm::DbOrm;
use crate::shared::events::{EventBus, Updates};

use super::super::args::RollbackArgs;
use super::super::detect;
use super::super::support::guard_destructive;

pub fn run(
    ctx: &Context,
    root: &Path,
    bus: &EventBus,
    orm_override: Option<DbOrm>,
    args: &RollbackArgs,
) -> Result<()> {
    let is_production = std::env::var("NODE_ENV").as_deref() == Ok("production");
    bus.run("db migrate rollback", |events| {
        guard_destructive(
            events,
            &format!("roll back the last {} migration(s)", args.step),
            args.yes,
            args.force,
            is_production,
        )?;
        let detected = detect::detect(ctx, root, orm_override)?;
        events.updated(format!(
            "Rolling back {} migration(s) ({})",
            args.step,
            detected.orm.as_str()
        ));
        run_for_orm(ctx, root, detected.orm, args.step, events)?;
        Ok(format!("Rolled back {} migration(s)", args.step))
    })
}

fn run_for_orm(ctx: &Context, root: &Path, orm: DbOrm, step: u32, events: &Updates) -> Result<()> {
    match orm {
        DbOrm::Typeorm => {
            for i in 1..=step {
                if step > 1 {
                    events.updated(format!("Reverting migration {i}/{step}"));
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

#[cfg(test)]
mod tests;
