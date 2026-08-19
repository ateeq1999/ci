//! `ci add validation` — self-contained: patches `main.ts` to register a
//! global `ValidationPipe`, installs `class-validator`/`class-transformer`
//! with the project's configured package manager.

use std::path::Path;

use anyhow::Result;

use crate::shared::context::Context;
use crate::shared::events::EventBus;

use super::patch;

const IMPORT_LINE: &str = "import { ValidationPipe } from '@nestjs/common';";
const PIPE_BLOCK: &str = "  app.useGlobalPipes(\n    new ValidationPipe({\n      whitelist: true,\n      forbidNonWhitelisted: true,\n      transform: true,\n    }),\n  );";

pub fn run(ctx: &Context, root: &Path, bus: &EventBus) -> Result<()> {
    bus.run("add validation", |events| {
        let package_manager = patch::detect_package_manager(ctx, root);
        events.updated(format!(
            "Installing class-validator and class-transformer with {}",
            package_manager.command()
        ));
        patch::install_dependencies(
            ctx,
            root,
            package_manager,
            &["class-validator", "class-transformer"],
        )?;

        let main_ts = root.join("src/main.ts");

        events.updated("Registering the global ValidationPipe in main.ts");
        let import_added = patch::insert_after(
            ctx,
            &main_ts,
            "import { NestFactory } from '@nestjs/core';",
            "import { ValidationPipe }",
            IMPORT_LINE,
        )?;
        let pipe_added = patch::insert_after(
            ctx,
            &main_ts,
            "const app = await NestFactory.create(AppModule);",
            "app.useGlobalPipes",
            PIPE_BLOCK,
        )?;

        Ok(if import_added || pipe_added {
            "Validation configured".to_string()
        } else {
            "Validation was already configured".to_string()
        })
    })
}

#[cfg(test)]
mod tests;
