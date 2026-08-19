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

#[test]
fn prisma_is_identical_to_fresh() {
    let (ctx, calls) = ctx();
    run(
        &ctx,
        root(),
        &listeners::bus(&ctx, root()),
        Some(DbOrm::Prisma),
        &destructive(),
    )
    .unwrap();

    assert_eq!(
        calls.borrow().as_slice(),
        ["npx prisma migrate reset --force"]
    );
}

#[test]
fn typeorm_is_not_implemented() {
    let (ctx, _calls) = ctx();
    let err = run(
        &ctx,
        root(),
        &listeners::bus(&ctx, root()),
        Some(DbOrm::Typeorm),
        &destructive(),
    )
    .unwrap_err();
    assert!(!err.to_string().is_empty());
}

#[test]
fn drizzle_reuses_the_fresh_drop_and_rebuild() {
    let fs = InMemoryFileSystem::default();
    fs.written.borrow_mut().insert(
        root().join(".env"),
        "DATABASE_URL=postgres://localhost/my_api\n".to_string(),
    );
    let commands = NoopCommandRunner::default();
    let calls = commands.calls.clone();
    let ctx = Context {
        fs: Box::new(fs),
        commands: Box::new(commands),
        ui: Box::new(ConsoleUi),
    };

    run(
        &ctx,
        root(),
        &listeners::bus(&ctx, root()),
        Some(DbOrm::Drizzle),
        &destructive(),
    )
    .unwrap();

    let calls = calls.borrow();
    assert_eq!(calls.len(), 3);
    assert!(calls[0].starts_with("node -e"));
    assert_eq!(calls[1], "npx drizzle-kit generate");
    assert_eq!(calls[2], "npx drizzle-kit migrate");
}
