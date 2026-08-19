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
/// (not a hand-typed copy of `main.ts`/`app.module.ts` that can silently
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
fn writes_logger_files_and_wires_app_module_and_main_ts() {
    let (ctx, calls) = ctx_from_real_init_output();
    let bus = crate::commands::add::listeners::bus(&ctx, root());

    run(&ctx, root(), &bus).unwrap();

    // No dependency installs — everything used ships in @nestjs/common.
    assert!(calls.borrow().is_empty());

    let service = read(&ctx, "src/logger/logger.service.ts");
    assert!(service.contains("export class AppLogger extends ConsoleLogger"));

    let module = read(&ctx, "src/logger/logger.module.ts");
    assert!(module.contains("export class LoggerModule"));
    assert!(module.contains("@Global()"));

    let app_module = read(&ctx, "src/app.module.ts");
    assert!(app_module.contains("import { LoggerModule } from './logger/logger.module';"));
    assert!(app_module.contains("LoggerModule,"));
    // Already-present DatabaseModule wiring must survive the patch untouched.
    assert!(app_module.contains("DatabaseModule,"));

    let main_ts = read(&ctx, "src/main.ts");
    assert!(main_ts.contains("import { AppLogger } from './logger/logger.service';"));
    assert!(main_ts.contains("app.useLogger(app.get(AppLogger));"));
    // The useLogger call lands before app.listen, not after.
    let use_logger_idx = main_ts.find("app.useLogger").unwrap();
    let listen_idx = main_ts.find("await app.listen(").unwrap();
    assert!(use_logger_idx < listen_idx);
}

#[test]
fn running_twice_does_not_duplicate_anything_or_clobber_the_service_file() {
    let (ctx, _calls) = ctx_from_real_init_output();
    let bus = crate::commands::add::listeners::bus(&ctx, root());

    run(&ctx, root(), &bus).unwrap();
    run(&ctx, root(), &bus).unwrap();

    let app_module = read(&ctx, "src/app.module.ts");
    assert_eq!(app_module.matches("LoggerModule,").count(), 1);
    assert_eq!(app_module.matches("import { LoggerModule }").count(), 1);

    let main_ts = read(&ctx, "src/main.ts");
    assert_eq!(main_ts.matches("app.useLogger").count(), 1);
    assert_eq!(main_ts.matches("import { AppLogger }").count(), 1);
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
    let bus = crate::commands::add::listeners::bus(&ctx, root());

    run(&ctx, root(), &bus).unwrap();
    run(&ctx, root(), &bus).unwrap();

    let messages = messages.borrow();
    assert!(
        messages
            .iter()
            .any(|m| m == "success: Logger was already configured")
    );
}

#[test]
fn stacks_after_validation_in_main_ts_instead_of_colliding_on_the_same_anchor() {
    let (ctx, _calls) = ctx_from_real_init_output();
    let bus = crate::commands::add::listeners::bus(&ctx, root());

    crate::commands::add::validation::run(&ctx, root(), &bus).unwrap();
    run(&ctx, root(), &bus).unwrap();

    let main_ts = read(&ctx, "src/main.ts");
    assert!(main_ts.contains("app.useGlobalPipes("));
    assert!(main_ts.contains("app.useLogger(app.get(AppLogger));"));
    // Both statements present and in a sane bootstrap order: pipe (right
    // after NestFactory.create) above useLogger (right before listen).
    let pipe_idx = main_ts.find("app.useGlobalPipes(").unwrap();
    let use_logger_idx = main_ts.find("app.useLogger").unwrap();
    let listen_idx = main_ts.find("await app.listen(").unwrap();
    assert!(pipe_idx < use_logger_idx);
    assert!(use_logger_idx < listen_idx);
}

#[test]
fn does_not_overwrite_a_hand_edited_logger_service() {
    let (ctx, _calls) = ctx_from_real_init_output();
    let bus = crate::commands::add::listeners::bus(&ctx, root());

    run(&ctx, root(), &bus).unwrap();

    let custom = "// hand-edited\nexport class AppLogger {}\n";
    ctx.fs
        .write_file(&root().join("src/logger/logger.service.ts"), custom)
        .unwrap();

    run(&ctx, root(), &bus).unwrap();

    assert_eq!(read(&ctx, "src/logger/logger.service.ts"), custom);
}
