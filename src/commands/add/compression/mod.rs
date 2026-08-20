//! `ci add compression` — self-contained: installs `@fastify/compress`
//! and registers it in `main.ts`'s bootstrap. Fastify-only (`init` no
//! longer scaffolds Express, so there's no other platform to support).
//! No `.env` change, no `app.module.ts` change — this is bootstrap-level
//! middleware, not a module.

use std::path::Path;

use anyhow::Result;

use crate::shared::context::Context;
use crate::shared::events::EventBus;

use super::patch;

const IMPORT_LINE: &str = "import compression from '@fastify/compress';";
const REGISTER_LINE: &str = "  await app.register(compression);";

pub fn run(ctx: &Context, root: &Path, bus: &EventBus) -> Result<()> {
    bus.run("add compression", |events| {
        let package_manager = patch::detect_package_manager(ctx, root);
        events.updated(format!(
            "Installing @fastify/compress with {}",
            package_manager.command()
        ));
        patch::install_dependencies(ctx, root, package_manager, &["@fastify/compress"])?;

        let main_ts = root.join("src/main.ts");

        events.updated("Registering compression in main.ts's bootstrap");
        let import_added = patch::insert_after_last_import(
            ctx,
            &main_ts,
            "import compression from '@fastify/compress'",
            IMPORT_LINE,
        )?;
        let register_added = patch::insert_before(
            ctx,
            &main_ts,
            "await app.listen(",
            "app.register(compression)",
            REGISTER_LINE,
        )?;

        Ok(if import_added || register_added {
            "Compression configured".to_string()
        } else {
            "Compression was already configured".to_string()
        })
    })
}

#[cfg(test)]
mod tests;
