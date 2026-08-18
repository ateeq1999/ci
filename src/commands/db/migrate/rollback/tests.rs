use std::path::Path;

use super::*;
use crate::commands::db::args::RollbackArgs;
use crate::commands::db::listeners;
use crate::shared::context::NoopCommandRunner;
use crate::shared::fs::InMemoryFileSystem;
use crate::shared::ui::ConsoleUi;

fn root() -> &'static Path {
    Path::new("proj")
}

fn rollback_args(step: u32) -> RollbackArgs {
    RollbackArgs {
        yes: true,
        force: false,
        step,
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
fn typeorm_reverts_step_times() {
    let (ctx, calls) = ctx();
    run(
        &ctx,
        root(),
        &listeners::bus(&ctx),
        Some(DbOrm::Typeorm),
        &rollback_args(3),
    )
    .unwrap();

    assert_eq!(calls.borrow().len(), 3);
    assert!(calls.borrow().iter().all(
        |c| c == "npx typeorm-ts-node-commonjs migration:revert -d src/database/data-source.ts"
    ));
}

#[test]
fn drizzle_and_prisma_are_not_supported() {
    for orm in [DbOrm::Drizzle, DbOrm::Prisma] {
        let (ctx, calls) = ctx();
        let err = run(
            &ctx,
            root(),
            &listeners::bus(&ctx),
            Some(orm),
            &rollback_args(1),
        )
        .unwrap_err();
        assert!(err.to_string().contains("isn't supported"), "orm = {orm:?}");
        assert!(calls.borrow().is_empty());
    }
}
