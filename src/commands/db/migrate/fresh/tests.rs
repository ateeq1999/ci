use std::path::Path;

use super::*;
use crate::commands::db::args::DestructiveArgs;
use crate::commands::db::listeners;
use crate::shared::context::NoopCommandRunner;
use crate::shared::fs::InMemoryFileSystem;
use crate::shared::ui::ConsoleUi;

fn root() -> &'static Path {
    Path::new("proj")
}

fn destructive() -> DestructiveArgs {
    DestructiveArgs {
        yes: true,
        force: false,
    }
}

fn ctx() -> (Context, std::rc::Rc<std::cell::RefCell<Vec<String>>>) {
    let commands = NoopCommandRunner::default();
    let calls = commands.calls.clone();
    (
        Context {
            fs: Box::new(InMemoryFileSystem::default()),
            commands: Box::new(commands),
            ui: Box::new(ConsoleUi),
        },
        calls,
    )
}

fn ctx_with_database_url(url: &str) -> (Context, std::rc::Rc<std::cell::RefCell<Vec<String>>>) {
    let fs = InMemoryFileSystem::default();
    fs.written
        .borrow_mut()
        .insert(root().join(".env"), format!("DATABASE_URL={url}\n"));
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

#[test]
fn prisma_resets_with_force() {
    let (ctx, calls) = ctx();
    run(&ctx, root(), &listeners::bus(&ctx), Some(DbOrm::Prisma), &destructive()).unwrap();

    assert_eq!(
        calls.borrow().as_slice(),
        ["npx prisma migrate reset --force"]
    );
}

#[test]
fn typeorm_drops_schema_then_runs() {
    let (ctx, calls) = ctx();
    run(&ctx, root(), &listeners::bus(&ctx), Some(DbOrm::Typeorm), &destructive()).unwrap();

    assert_eq!(
        calls.borrow().as_slice(),
        [
            "npx typeorm-ts-node-commonjs schema:drop -d src/database/data-source.ts",
            "npx typeorm-ts-node-commonjs migration:run -d src/database/data-source.ts",
        ]
    );
}

#[test]
fn drizzle_drops_schema_then_regenerates() {
    let (ctx, calls) = ctx_with_database_url("postgres://user:pass@localhost:5432/my_api");
    run(&ctx, root(), &listeners::bus(&ctx), Some(DbOrm::Drizzle), &destructive()).unwrap();

    let calls = calls.borrow();
    assert_eq!(calls.len(), 3);
    assert!(calls[0].starts_with("node -e"));
    assert!(
        calls[0].contains("require(\"pg\")"),
        "should use the pg driver by default"
    );
    assert!(calls[0].contains("postgres://user:pass@localhost:5432/my_api"));
    assert!(calls[0].contains("DROP SCHEMA IF EXISTS public CASCADE"));
    assert!(calls[0].contains("DROP SCHEMA IF EXISTS drizzle CASCADE"));
    assert_eq!(calls[1], "npx drizzle-kit generate");
    assert_eq!(calls[2], "npx drizzle-kit migrate");
}

#[test]
fn drizzle_postgres_js_driver_uses_the_postgres_js_script() {
    let fs = InMemoryFileSystem::default();
    fs.written.borrow_mut().insert(
        root().join(".env"),
        "DATABASE_URL=postgres://localhost/my_api\n".to_string(),
    );
    fs.written.borrow_mut().insert(
        root().join("ci/config.json"),
        r#"{"ciVersion":"0.1.1","orm":"drizzle","driver":"postgres-js","packageManager":"npm"}"#
            .to_string(),
    );
    let commands = NoopCommandRunner::default();
    let calls = commands.calls.clone();
    let ctx = Context {
        fs: Box::new(fs),
        commands: Box::new(commands),
        ui: Box::new(ConsoleUi),
    };

    run(&ctx, root(), &listeners::bus(&ctx), None, &destructive()).unwrap();

    assert!(calls.borrow()[0].contains("require(\"postgres\")"));
}

#[test]
fn drizzle_errors_clearly_without_a_env_file() {
    let (ctx, calls) = ctx();
    let err = run(&ctx, root(), &listeners::bus(&ctx), Some(DbOrm::Drizzle), &destructive())
        .unwrap_err();

    assert!(err.to_string().contains(".env"));
    assert!(err.to_string().contains("not found"));
    assert!(calls.borrow().is_empty());
}
