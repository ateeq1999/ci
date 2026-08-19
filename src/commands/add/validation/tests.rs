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
/// (not a hand-typed copy of `main.ts`/`package.json` that can silently
/// drift from what `init` actually produces).
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
fn configures_main_ts_and_installs_with_npm_by_default() {
    let (ctx, calls) = ctx_from_real_init_output();
    let bus = crate::commands::add::listeners::bus(&ctx);

    run(&ctx, root(), &bus).unwrap();

    let main_ts = read(&ctx, "src/main.ts");
    assert!(main_ts.contains("import { ValidationPipe } from '@nestjs/common';"));
    assert!(main_ts.contains("app.useGlobalPipes("));
    assert!(main_ts.contains("whitelist: true"));

    assert_eq!(
        calls.borrow().as_slice(),
        ["npm install class-validator class-transformer"]
    );

    // No example DTO — `ci add validation` only wires the pipe + installs.
    assert!(
        ctx.fs
            .try_read_to_string(&root().join("src/common/dto/example.dto.ts"))
            .unwrap()
            .is_none()
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
        r#"{"ciVersion":"0.1.2","orm":"drizzle","packageManager":"pnpm"}"#.to_string(),
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
        ["pnpm add class-validator class-transformer"]
    );
}

#[test]
fn running_twice_does_not_duplicate_the_pipe() {
    let (ctx, _calls) = ctx_from_real_init_output();
    let bus = crate::commands::add::listeners::bus(&ctx);

    run(&ctx, root(), &bus).unwrap();
    run(&ctx, root(), &bus).unwrap();

    let main_ts = read(&ctx, "src/main.ts");
    assert_eq!(main_ts.matches("app.useGlobalPipes(").count(), 1);
    assert_eq!(main_ts.matches("import { ValidationPipe }").count(), 1);
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
            .any(|m| m == "success: Validation was already configured")
    );
}
