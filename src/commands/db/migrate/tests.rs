use std::path::Path;

use super::*;
use crate::commands::db::args::MigrateArgs;
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

#[test]
fn bare_migrate_maps_to_the_right_command_per_orm() {
    for (orm, expected) in [
        (DbOrm::Drizzle, "npx drizzle-kit migrate"),
        (DbOrm::Prisma, "npx prisma migrate deploy"),
        (
            DbOrm::Typeorm,
            "npx typeorm-ts-node-commonjs migration:run -d src/database/data-source.ts",
        ),
    ] {
        let (ctx, calls) = ctx();
        run(
            &ctx,
            Path::new("proj"),
            &listeners::bus(&ctx),
            &MigrateArgs {
                action: None,
                orm: Some(orm),
            },
        )
        .unwrap();

        assert_eq!(calls.borrow().as_slice(), [expected], "orm = {orm:?}");
    }
}
