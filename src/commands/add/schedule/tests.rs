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
fn ctx_from_real_init_output() -> (Context, std::rc::Rc<std::cell::RefCell<Vec<String>>>) {
    let fs = InMemoryFileSystem::default();
    for (path, contents) in
        templates::starter_files("my-api", DbOrm::Drizzle, DrizzleDriver::Pg).unwrap()
    {
        fs.written
            .borrow_mut()
            .insert(root().join(path), contents);
    }
    let commands = NoopCommandRunner::default();
    let calls = commands.calls.clone();
    (
        Context {
            fs: Box::new(fs),
            commands: Box::new(commands),
            ui: Box::new(ConsoleUi),
        },
        calls,
    )
}

fn read(ctx: &Context, path: &str) -> String {
    ctx.fs
        .try_read_to_string(&root().join(path))
        .unwrap()
        .unwrap()
}

#[test]
fn configures_app_module_and_installs_with_npm_by_default() {
    let (ctx, calls) = ctx_from_real_init_output();
    let bus = crate::commands::add::listeners::bus(&ctx);

    run(&ctx, root(), &bus).unwrap();

    let app_module = read(&ctx, "src/app.module.ts");
    assert!(app_module.contains("import { ScheduleModule } from '@nestjs/schedule';"));
    assert!(app_module.contains("ScheduleModule.forRoot(),"));
    // Already-present DatabaseModule wiring must survive the patch untouched.
    assert!(app_module.contains("DatabaseModule,"));

    assert_eq!(
        calls.borrow().as_slice(),
        ["npm install @nestjs/schedule"]
    );
}

#[test]
fn installs_with_the_project_configured_package_manager() {
    let fs = InMemoryFileSystem::default();
    for (path, contents) in
        templates::starter_files("my-api", DbOrm::Drizzle, DrizzleDriver::Pg).unwrap()
    {
        fs.written
            .borrow_mut()
            .insert(root().join(path), contents);
    }
    fs.written.borrow_mut().insert(
        root().join("ci/config.json"),
        r#"{"ciVersion":"0.1.2","orm":"drizzle","packageManager":"yarn"}"#.to_string(),
    );
    let commands = NoopCommandRunner::default();
    let calls = commands.calls.clone();
    let ctx = Context {
        fs: Box::new(fs),
        commands: Box::new(commands),
        ui: Box::new(ConsoleUi),
    };
    let bus = crate::commands::add::listeners::bus(&ctx);

    run(&ctx, root(), &bus).unwrap();

    assert_eq!(
        calls.borrow().as_slice(),
        ["yarn add @nestjs/schedule"]
    );
}

#[test]
fn running_twice_does_not_duplicate_the_module() {
    let (ctx, _calls) = ctx_from_real_init_output();
    let bus = crate::commands::add::listeners::bus(&ctx);

    run(&ctx, root(), &bus).unwrap();
    run(&ctx, root(), &bus).unwrap();

    let app_module = read(&ctx, "src/app.module.ts");
    assert_eq!(app_module.matches("ScheduleModule.forRoot()").count(), 1);
    assert_eq!(app_module.matches("import { ScheduleModule }").count(), 1);
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
            .any(|m| m == "success: Task scheduling was already configured")
    );
}

#[test]
fn stacks_after_cache_instead_of_colliding_on_the_same_anchor() {
    let (ctx, _calls) = ctx_from_real_init_output();
    let bus = crate::commands::add::listeners::bus(&ctx);

    crate::commands::add::cache::run(&ctx, root(), &bus).unwrap();
    run(&ctx, root(), &bus).unwrap();

    let app_module = read(&ctx, "src/app.module.ts");
    assert!(app_module.contains("import { CacheModule } from '@nestjs/cache-manager';"));
    assert!(app_module.contains("import { ScheduleModule } from '@nestjs/schedule';"));
    // ScheduleModule's import must land after CacheModule's, not before it.
    let cache_idx = app_module.find("import { CacheModule }").unwrap();
    let schedule_idx = app_module.find("import { ScheduleModule }").unwrap();
    assert!(cache_idx < schedule_idx);
}
