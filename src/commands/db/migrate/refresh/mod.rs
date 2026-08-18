//! `ci db migrate refresh` — self-contained except for one deliberate
//! exception: its Drizzle case calls `fresh::run_for_orm` rather than
//! duplicating it (see the comment on that function for why they're the
//! same operation for Drizzle).

use std::path::Path;

use anyhow::{Result, bail};

use crate::shared::context::Context;
use crate::shared::db_orm::DbOrm;
use crate::shared::events::EventBus;

use super::super::args::DestructiveArgs;
use super::super::detect;
use super::super::support::guard_destructive;
use super::fresh;

pub fn run(
    ctx: &Context,
    root: &Path,
    bus: &EventBus,
    orm_override: Option<DbOrm>,
    args: &DestructiveArgs,
) -> Result<()> {
    let is_production = std::env::var("NODE_ENV").as_deref() == Ok("production");
    bus.run("db migrate refresh", |events| {
        guard_destructive(
            events,
            "roll back and re-apply every migration",
            args.yes,
            args.force,
            is_production,
        )?;
        let detected = detect::detect(ctx, root, orm_override)?;
        events.updated(format!("Refreshing the database ({})", detected.orm.as_str()));

        match detected.orm {
            DbOrm::Prisma => {
                events.updated("Resetting the database (drop, re-apply, re-seed)");
                ctx.commands
                    .run("npx", &["prisma", "migrate", "reset", "--force"], root)?;
            }
            DbOrm::Drizzle => {
                fresh::run_for_orm(ctx, root, detected.orm, detected.driver, events)?;
            }
            DbOrm::Typeorm => bail!(
                "`ci db migrate refresh` isn't implemented for TypeORM yet — safely reverting \
                 *all* migrations needs to know how many exist first; only \
                 `migrate rollback --step N` (explicit count) is implemented so far."
            ),
        }

        Ok("Database refreshed".to_string())
    })
}

#[cfg(test)]
mod tests;
