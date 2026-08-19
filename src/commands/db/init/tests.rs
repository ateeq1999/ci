use std::path::Path;

use super::*;
use crate::commands::db::args::RunArgs;
use crate::commands::db::listeners;
use crate::shared::context::NoopCommandRunner;
use crate::shared::fs::InMemoryFileSystem;
use crate::shared::ui::ConsoleUi;

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

fn run_args(orm: DbOrm) -> RunArgs {
    RunArgs { orm: Some(orm) }
}

#[test]
fn drizzle_generates_then_migrates() {
    let (ctx, calls) = ctx();
    run(&ctx, Path::new("proj"), &listeners::bus(&ctx, Path::new("proj")), &run_args(DbOrm::Drizzle)).unwrap();

    assert_eq!(
        calls.borrow().as_slice(),
        ["npx drizzle-kit generate", "npx drizzle-kit migrate"]
    );
}

#[test]
fn prisma_runs_migrate_dev() {
    let (ctx, calls) = ctx();
    run(&ctx, Path::new("proj"), &listeners::bus(&ctx, Path::new("proj")), &run_args(DbOrm::Prisma)).unwrap();

    assert_eq!(
        calls.borrow().as_slice(),
        ["npx prisma migrate dev --name init"]
    );
}

#[test]
fn typeorm_generates_then_runs() {
    let (ctx, calls) = ctx();
    run(&ctx, Path::new("proj"), &listeners::bus(&ctx, Path::new("proj")), &run_args(DbOrm::Typeorm)).unwrap();

    assert_eq!(
        calls.borrow().as_slice(),
        [
            "npx typeorm-ts-node-commonjs migration:generate \
             src/database/migrations/init -d src/database/data-source.ts",
            "npx typeorm-ts-node-commonjs migration:run -d src/database/data-source.ts",
        ]
    );
}
