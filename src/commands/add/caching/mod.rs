//! `ci add caching` — self-contained: patches `app.module.ts` to register
//! `CacheModule` backed by Redis (the approved default — not the docs'
//! bare in-memory `register()`), adds the required dependencies, and adds
//! `REDIS_URL` to `.env`/`.env.example`.

use std::path::Path;

use anyhow::Result;

use crate::shared::context::Context;
use crate::shared::events::EventBus;

use super::patch;

const IMPORT_LINES: &str =
    "import { CacheModule } from '@nestjs/cache-manager';\nimport { KeyvRedis } from '@keyv/redis';";

const IMPORTS_ARRAY_ITEM: &str = "CacheModule.registerAsync({\n      isGlobal: true,\n      useFactory: async () => ({\n        stores: [new KeyvRedis(process.env.REDIS_URL ?? 'redis://localhost:6379')],\n      }),\n    }),";

const REDIS_URL_LINE: &str = "REDIS_URL=redis://localhost:6379";

pub fn run(ctx: &Context, root: &Path, bus: &EventBus) -> Result<()> {
    bus.run("add caching", |events| {
        events.updated("Adding @nestjs/cache-manager, cache-manager, @keyv/redis");
        patch::add_dependencies(
            ctx,
            root,
            &[
                ("@nestjs/cache-manager", "^2.2.2"),
                ("cache-manager", "^5.7.6"),
                ("@keyv/redis", "^2.8.4"),
            ],
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

        events.updated("Registering CacheModule (Redis-backed) in app.module.ts");
        let import_added = patch::insert_after(
            ctx,
            &app_module_ts,
            "import { DatabaseModule } from './database/database.module';",
            "import { CacheModule }",
            IMPORT_LINES,
        )?;
        let module_added = patch::insert_into_array(
            ctx,
            &app_module_ts,
            "imports: [",
            "CacheModule.registerAsync",
            IMPORTS_ARRAY_ITEM,
        )?;

        Ok(if import_added || module_added || env_example_updated {
            "Caching configured".to_string()
        } else {
            "Caching was already configured".to_string()
        })
    })
}

#[cfg(test)]
mod tests;
