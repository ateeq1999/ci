//! `ci add schedule` — self-contained: registers `ScheduleModule` so
//! `@Cron()`/`@Interval()`/`@Timeout()` decorators become usable, and
//! installs the required dependency with the project's configured
//! package manager. No `.env` change — nothing here is environment-driven.

use std::path::Path;

use anyhow::Result;

use crate::shared::context::Context;
use crate::shared::events::EventBus;

use super::patch;

const IMPORT_LINE: &str = "import { ScheduleModule } from '@nestjs/schedule';";

const IMPORTS_ARRAY_ITEM: &str = "ScheduleModule.forRoot(),";

pub fn run(ctx: &Context, root: &Path, bus: &EventBus) -> Result<()> {
    bus.run("add schedule", |events| {
        let package_manager = patch::detect_package_manager(ctx, root);
        events.updated(format!(
            "Installing @nestjs/schedule with {}",
            package_manager.command()
        ));
        patch::install_dependencies(ctx, root, package_manager, &["@nestjs/schedule"])?;

        let app_module_ts = root.join("src/app.module.ts");

        events.updated("Registering ScheduleModule in app.module.ts");
        let import_added = patch::insert_after_last_import(
            ctx,
            &app_module_ts,
            "import { ScheduleModule }",
            IMPORT_LINE,
        )?;
        let module_added = patch::insert_into_array(
            ctx,
            &app_module_ts,
            "imports: [",
            "ScheduleModule.forRoot",
            IMPORTS_ARRAY_ITEM,
        )?;

        Ok(if import_added || module_added {
            "Task scheduling configured".to_string()
        } else {
            "Task scheduling was already configured".to_string()
        })
    })
}

#[cfg(test)]
mod tests;
