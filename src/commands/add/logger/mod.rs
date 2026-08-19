//! `ci add logger` — self-contained: writes a standalone `LoggerModule`
//! (matching `db`'s own `@Global()`-module shape) that wraps the built-in
//! `ConsoleLogger` for DI, registers it in `app.module.ts`, and wires it
//! up as the app's logger in `main.ts`. No new dependencies — everything
//! it uses (`ConsoleLogger`/`Injectable`/`Global`/`Module`) already ships
//! in `@nestjs/common`, so unlike every other `add` subcommand this one
//! never shells out to a package manager.

use std::path::Path;

use anyhow::Result;

use crate::shared::context::Context;
use crate::shared::events::EventBus;

use super::patch;

const LOGGER_SERVICE_TS: &str =
    include_str!("../../../../templates/add/logger/logger.service.ts");
const LOGGER_MODULE_TS: &str = include_str!("../../../../templates/add/logger/logger.module.ts");

const APP_MODULE_IMPORT_LINE: &str = "import { LoggerModule } from './logger/logger.module';";
const IMPORTS_ARRAY_ITEM: &str = "LoggerModule,";

const MAIN_TS_IMPORT_LINE: &str = "import { AppLogger } from './logger/logger.service';";
const USE_LOGGER_LINE: &str = "  app.useLogger(app.get(AppLogger));";

pub fn run(ctx: &Context, root: &Path, bus: &EventBus) -> Result<()> {
    bus.run("add logger", |events| {
        events.updated("Writing src/logger/logger.service.ts and logger.module.ts");
        let service_written = patch::write_file_if_absent(
            ctx,
            &root.join("src/logger/logger.service.ts"),
            LOGGER_SERVICE_TS,
        )?;
        let module_written = patch::write_file_if_absent(
            ctx,
            &root.join("src/logger/logger.module.ts"),
            LOGGER_MODULE_TS,
        )?;

        let app_module_ts = root.join("src/app.module.ts");
        events.updated("Registering LoggerModule in app.module.ts");
        let app_module_import_added = patch::insert_after_last_import(
            ctx,
            &app_module_ts,
            "import { LoggerModule }",
            APP_MODULE_IMPORT_LINE,
        )?;
        let app_module_entry_added = patch::insert_into_array(
            ctx,
            &app_module_ts,
            "imports: [",
            "LoggerModule,",
            IMPORTS_ARRAY_ITEM,
        )?;

        let main_ts = root.join("src/main.ts");
        events.updated("Wiring AppLogger into main.ts's bootstrap");
        let main_ts_import_added = patch::insert_after_last_import(
            ctx,
            &main_ts,
            "import { AppLogger }",
            MAIN_TS_IMPORT_LINE,
        )?;
        let use_logger_added = patch::insert_before(
            ctx,
            &main_ts,
            "await app.listen(",
            "app.useLogger",
            USE_LOGGER_LINE,
        )?;

        Ok(
            if service_written
                || module_written
                || app_module_import_added
                || app_module_entry_added
                || main_ts_import_added
                || use_logger_added
            {
                "Logger configured".to_string()
            } else {
                "Logger was already configured".to_string()
            },
        )
    })
}

#[cfg(test)]
mod tests;
