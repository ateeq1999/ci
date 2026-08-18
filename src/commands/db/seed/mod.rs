//! `ci db seed` — self-contained, though today it only has one thing to
//! say: no seed script template exists yet for any ORM.

use std::path::Path;

use anyhow::{Result, bail};

use crate::shared::context::Context;
use crate::shared::events::EventBus;

use super::args::RunArgs;
use super::detect;

pub fn run(ctx: &Context, root: &Path, bus: &EventBus, args: &RunArgs) -> Result<()> {
    bus.run("db seed", |_events| {
        detect::detect(ctx, root, args.orm)?;
        bail!(
            "`ci db seed` isn't implemented yet — no seed script template exists for \
             any ORM yet. See db.md's Gap 2/3."
        )
    })
}

#[cfg(test)]
mod tests;
