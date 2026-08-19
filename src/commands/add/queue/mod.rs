//! `ci add queue` — self-contained: patches `app.module.ts` to register
//! `BullModule` with a Redis connection (BullMQ, matching `cache`'s
//! already-Redis-backed precedent over the older `@nestjs/bull`/Bull),
//! installs the required dependencies with the project's configured
//! package manager, and adds `REDIS_URL` to `.env`/`.env.example`
//! (sharing `cache`'s marker so running both doesn't duplicate the line).

use std::path::Path;

use anyhow::Result;

use crate::shared::context::Context;
use crate::shared::events::EventBus;

use super::patch;

const IMPORT_LINES: &str =
    "import { BullModule } from '@nestjs/bullmq';\nimport IORedis from 'ioredis';";

const IMPORTS_ARRAY_ITEM: &str = "BullModule.forRoot({\n      connection: new IORedis(process.env.REDIS_URL ?? 'redis://localhost:6379', {\n        maxRetriesPerRequest: null,\n      }),\n    }),";

const REDIS_URL_LINE: &str = "REDIS_URL=redis://localhost:6379";

pub fn run(ctx: &Context, root: &Path, bus: &EventBus) -> Result<()> {
    bus.run("add queue", |events| {
        let package_manager = patch::detect_package_manager(ctx, root);
        events.updated(format!(
            "Installing @nestjs/bullmq, bullmq, ioredis with {}",
            package_manager.command()
        ));
        patch::install_dependencies(
            ctx,
            root,
            package_manager,
            &["@nestjs/bullmq", "bullmq", "ioredis"],
        )?;

        events.updated("Adding REDIS_URL to .env and .env.example");
        let env_example_updated = patch::append_line(
            ctx,
            &root.join(".env.example"),
            "REDIS_URL",
            REDIS_URL_LINE,
        )?;
        patch::append_line(ctx, &root.join(".env"), "REDIS_URL", REDIS_URL_LINE)?;

        let app_module_ts = root.join("src/app.module.ts");

        events.updated("Registering BullModule (Redis-backed) in app.module.ts");
        let import_added = patch::insert_after_last_import(
            ctx,
            &app_module_ts,
            "import { BullModule }",
            IMPORT_LINES,
        )?;
        let module_added = patch::insert_into_array(
            ctx,
            &app_module_ts,
            "imports: [",
            "BullModule.forRoot",
            IMPORTS_ARRAY_ITEM,
        )?;

        Ok(if import_added || module_added || env_example_updated {
            "Queues configured".to_string()
        } else {
            "Queues were already configured".to_string()
        })
    })
}

#[cfg(test)]
mod tests;
