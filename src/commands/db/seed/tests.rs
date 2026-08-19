use std::path::Path;

use super::*;
use crate::commands::db::args::RunArgs;
use crate::commands::db::listeners;
use crate::shared::context::NoopCommandRunner;
use crate::shared::db_orm::DbOrm;
use crate::shared::fs::InMemoryFileSystem;
use crate::shared::ui::ConsoleUi;

#[test]
fn not_implemented_for_any_orm() {
    for orm in [DbOrm::Drizzle, DbOrm::Typeorm, DbOrm::Prisma] {
        let commands = NoopCommandRunner::default();
        let calls = commands.calls.clone();
        let ctx = Context {
            fs: Box::new(InMemoryFileSystem::default()),
            commands: Box::new(commands),
            ui: Box::new(ConsoleUi),
        };

        let err = run(
            &ctx,
            Path::new("proj"),
            &listeners::bus(&ctx, Path::new("proj")),
            &RunArgs { orm: Some(orm) },
        )
        .unwrap_err();

        assert!(err.to_string().contains("isn't implemented yet"));
        assert!(calls.borrow().is_empty());
    }
}
