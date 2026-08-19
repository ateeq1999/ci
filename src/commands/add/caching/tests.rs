use std::path::Path;

use super::*;
use crate::commands::init::templates;
use crate::shared::context::NoopCommandRunner;
use crate::shared::db_orm::{DbOrm, DrizzleDriver};
use crate::shared::fs::InMemoryFileSystem;
use crate::shared::ui::{ConsoleUi, RecordingUi};

fn root() -> &'static Path {
    Path::new("proj")
}

/// Seeds an `InMemoryFileSystem` with `ci init`'s *real* rendered output
/// (not a hand-typed copy of `app.module.ts`/`package.json` that can
/// silently drift from what `init` actually produces).
fn ctx_from_real_init_output() -> Context {
    let fs = InMemoryFileSystem::default();
    for (path, contents) in
        templates::starter_files("my-api", DbOrm::Drizzle, DrizzleDriver::Pg).unwrap()
    {
        fs.written
            .borrow_mut()
            .insert(root().join(path), contents);
    }
    Context {
        fs: Box::new(fs),
        commands: Box::new(NoopCommandRunner::default()),
        ui: Box::new(ConsoleUi),
    }
}

fn read(ctx: &Context, path: &str) -> String {
    ctx.fs
        .try_read_to_string(&root().join(path))
        .unwrap()
        .unwrap()
}

#[test]
fn configures_app_module_adds_deps_and_redis_url() {
    let ctx = ctx_from_real_init_output();
    let bus = crate::commands::add::listeners::bus(&ctx);

    run(&ctx, root(), &bus).unwrap();

    let app_module = read(&ctx, "src/app.module.ts");
    assert!(app_module.contains("import { CacheModule } from '@nestjs/cache-manager';"));
    assert!(app_module.contains("import { KeyvRedis } from '@keyv/redis';"));
    assert!(app_module.contains("CacheModule.registerAsync({"));
    assert!(app_module.contains("new KeyvRedis(process.env.REDIS_URL"));
    // Already-present DatabaseModule wiring must survive the patch untouched.
    assert!(app_module.contains("DatabaseModule,"));

    let package_json = read(&ctx, "package.json");
    let parsed: serde_json::Value = serde_json::from_str(&package_json).unwrap();
    assert!(parsed["dependencies"]["@nestjs/cache-manager"].is_string());
    assert!(parsed["dependencies"]["cache-manager"].is_string());
    assert!(parsed["dependencies"]["@keyv/redis"].is_string());

    let env = read(&ctx, ".env");
    let env_example = read(&ctx, ".env.example");
    assert!(env.contains("REDIS_URL=redis://localhost:6379"));
    assert!(env_example.contains("REDIS_URL=redis://localhost:6379"));
}

#[test]
fn running_twice_does_not_duplicate_the_module() {
    let ctx = ctx_from_real_init_output();
    let bus = crate::commands::add::listeners::bus(&ctx);

    run(&ctx, root(), &bus).unwrap();
    run(&ctx, root(), &bus).unwrap();

    let app_module = read(&ctx, "src/app.module.ts");
    assert_eq!(app_module.matches("CacheModule.registerAsync").count(), 1);
    assert_eq!(app_module.matches("import { CacheModule }").count(), 1);

    let env = read(&ctx, ".env");
    assert_eq!(env.matches("REDIS_URL=").count(), 1);
}

#[test]
fn reports_already_configured_on_second_run() {
    let ui = RecordingUi::default();
    let messages = ui.messages.clone();
    let fs = InMemoryFileSystem::default();
    for (path, contents) in
        templates::starter_files("my-api", DbOrm::Drizzle, DrizzleDriver::Pg).unwrap()
    {
        fs.written
            .borrow_mut()
            .insert(root().join(path), contents);
    }
    let ctx = Context {
        fs: Box::new(fs),
        commands: Box::new(NoopCommandRunner::default()),
        ui: Box::new(ui),
    };
    let bus = crate::commands::add::listeners::bus(&ctx);

    run(&ctx, root(), &bus).unwrap();
    run(&ctx, root(), &bus).unwrap();

    let messages = messages.borrow();
    assert!(
        messages
            .iter()
            .any(|m| m == "success: Caching was already configured")
    );
}
