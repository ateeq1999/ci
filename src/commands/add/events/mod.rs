//! `ci add events` — self-contained: installs `@nestjs/event-emitter` and
//! registers `EventEmitterModule.forRoot()` in `app.module.ts`, so
//! `EventEmitter2` injection and `@OnEvent()` listeners become usable
//! project-wide. No `.env` change — nothing here is environment-driven.

use std::path::Path;

use anyhow::Result;

use crate::shared::context::Context;
use crate::shared::events::EventBus;

use super::patch;

const IMPORT_LINE: &str = "import { EventEmitterModule } from '@nestjs/event-emitter';";

const IMPORTS_ARRAY_ITEM: &str = "EventEmitterModule.forRoot(),";

pub fn run(ctx: &Context, root: &Path, bus: &EventBus) -> Result<()> {
    bus.run("add events", |events| {
        let package_manager = patch::detect_package_manager(ctx, root);
        events.updated(format!(
            "Installing @nestjs/event-emitter with {}",
            package_manager.command()
        ));
        patch::install_dependencies(ctx, root, package_manager, &["@nestjs/event-emitter"])?;

        let app_module_ts = root.join("src/app.module.ts");

        events.updated("Registering EventEmitterModule in app.module.ts");
        let import_added = patch::insert_after_last_import(
            ctx,
            &app_module_ts,
            "import { EventEmitterModule }",
            IMPORT_LINE,
        )?;
        let module_added = patch::insert_into_array(
            ctx,
            &app_module_ts,
            "imports: [",
            "EventEmitterModule.forRoot",
            IMPORTS_ARRAY_ITEM,
        )?;

        Ok(if import_added || module_added {
            "Events configured".to_string()
        } else {
            "Events were already configured".to_string()
        })
    })
}

#[cfg(test)]
mod tests;
