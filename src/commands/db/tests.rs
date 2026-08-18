use super::args::{Command, DestructiveArgs, MigrateAction, MigrateArgs, RollbackArgs, RunArgs};
use super::*;
use crate::context::NoopCommandRunner;
use crate::fs::InMemoryFileSystem;

fn ctx() -> (Context, std::rc::Rc<std::cell::RefCell<Vec<String>>>) {
    let commands = NoopCommandRunner::default();
    let calls = commands.calls.clone();
    (
        Context {
            fs: Box::new(InMemoryFileSystem::default()),
            commands: Box::new(commands),
        },
        calls,
    )
}

fn run_args(orm: DbOrm) -> RunArgs {
    RunArgs { orm: Some(orm) }
}

fn migrate_args(orm: DbOrm, action: Option<MigrateAction>) -> MigrateArgs {
    MigrateArgs {
        action,
        orm: Some(orm),
    }
}

fn destructive() -> DestructiveArgs {
    DestructiveArgs {
        yes: true,
        force: false,
    }
}

fn rollback(step: u32) -> RollbackArgs {
    RollbackArgs {
        yes: true,
        force: false,
        step,
    }
}

#[test]
fn init_drizzle_generates_then_migrates() {
    let (ctx, calls) = ctx();
    run(
        &Args {
            command: Command::Init(run_args(DbOrm::Drizzle)),
        },
        &ctx,
    )
    .unwrap();

    assert_eq!(
        calls.borrow().as_slice(),
        ["npx drizzle-kit generate", "npx drizzle-kit migrate"]
    );
}

#[test]
fn init_prisma_runs_migrate_dev() {
    let (ctx, calls) = ctx();
    run(
        &Args {
            command: Command::Init(run_args(DbOrm::Prisma)),
        },
        &ctx,
    )
    .unwrap();

    assert_eq!(
        calls.borrow().as_slice(),
        ["npx prisma migrate dev --name init"]
    );
}

#[test]
fn init_typeorm_generates_then_runs() {
    let (ctx, calls) = ctx();
    run(
        &Args {
            command: Command::Init(run_args(DbOrm::Typeorm)),
        },
        &ctx,
    )
    .unwrap();

    assert_eq!(
        calls.borrow().as_slice(),
        [
            "npx typeorm-ts-node-commonjs migration:generate \
             src/database/migrations/init -d src/database/data-source.ts",
            "npx typeorm-ts-node-commonjs migration:run -d src/database/data-source.ts",
        ]
    );
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
            &Args {
                command: Command::Migrate(migrate_args(orm, None)),
            },
            &ctx,
        )
        .unwrap();

        assert_eq!(calls.borrow().as_slice(), [expected], "orm = {orm:?}");
    }
}

#[test]
fn migrate_fresh_prisma_resets_with_force() {
    let (ctx, calls) = ctx();
    run(
        &Args {
            command: Command::Migrate(migrate_args(
                DbOrm::Prisma,
                Some(MigrateAction::Fresh(destructive())),
            )),
        },
        &ctx,
    )
    .unwrap();

    assert_eq!(
        calls.borrow().as_slice(),
        ["npx prisma migrate reset --force"]
    );
}

#[test]
fn migrate_fresh_typeorm_drops_schema_then_runs() {
    let (ctx, calls) = ctx();
    run(
        &Args {
            command: Command::Migrate(migrate_args(
                DbOrm::Typeorm,
                Some(MigrateAction::Fresh(destructive())),
            )),
        },
        &ctx,
    )
    .unwrap();

    assert_eq!(
        calls.borrow().as_slice(),
        [
            "npx typeorm-ts-node-commonjs schema:drop -d src/database/data-source.ts",
            "npx typeorm-ts-node-commonjs migration:run -d src/database/data-source.ts",
        ]
    );
}

#[test]
fn migrate_fresh_drizzle_errors_clearly() {
    let (ctx, calls) = ctx();
    let err = run(
        &Args {
            command: Command::Migrate(migrate_args(
                DbOrm::Drizzle,
                Some(MigrateAction::Fresh(destructive())),
            )),
        },
        &ctx,
    )
    .unwrap_err();

    assert!(err.to_string().contains("isn't supported for Drizzle"));
    assert!(calls.borrow().is_empty());
}

#[test]
fn migrate_refresh_prisma_is_identical_to_fresh() {
    let (ctx, calls) = ctx();
    run(
        &Args {
            command: Command::Migrate(migrate_args(
                DbOrm::Prisma,
                Some(MigrateAction::Refresh(destructive())),
            )),
        },
        &ctx,
    )
    .unwrap();

    assert_eq!(
        calls.borrow().as_slice(),
        ["npx prisma migrate reset --force"]
    );
}

#[test]
fn migrate_refresh_typeorm_and_drizzle_are_not_implemented() {
    for orm in [DbOrm::Typeorm, DbOrm::Drizzle] {
        let (ctx, _calls) = ctx();
        let err = run(
            &Args {
                command: Command::Migrate(migrate_args(
                    orm,
                    Some(MigrateAction::Refresh(destructive())),
                )),
            },
            &ctx,
        )
        .unwrap_err();
        assert!(!err.to_string().is_empty(), "orm = {orm:?}");
    }
}

#[test]
fn migrate_rollback_typeorm_reverts_step_times() {
    let (ctx, calls) = ctx();
    run(
        &Args {
            command: Command::Migrate(migrate_args(
                DbOrm::Typeorm,
                Some(MigrateAction::Rollback(rollback(3))),
            )),
        },
        &ctx,
    )
    .unwrap();

    assert_eq!(calls.borrow().len(), 3);
    assert!(
        calls
            .borrow()
            .iter()
            .all(|c| c == "npx typeorm-ts-node-commonjs migration:revert -d src/database/data-source.ts")
    );
}

#[test]
fn migrate_rollback_drizzle_and_prisma_are_not_supported() {
    for orm in [DbOrm::Drizzle, DbOrm::Prisma] {
        let (ctx, calls) = ctx();
        let err = run(
            &Args {
                command: Command::Migrate(migrate_args(
                    orm,
                    Some(MigrateAction::Rollback(rollback(1))),
                )),
            },
            &ctx,
        )
        .unwrap_err();
        assert!(err.to_string().contains("isn't supported"), "orm = {orm:?}");
        assert!(calls.borrow().is_empty());
    }
}

#[test]
fn seed_is_not_implemented_for_any_orm() {
    for orm in [DbOrm::Drizzle, DbOrm::Typeorm, DbOrm::Prisma] {
        let (ctx, calls) = ctx();
        let err = run(
            &Args {
                command: Command::Seed(run_args(orm)),
            },
            &ctx,
        )
        .unwrap_err();
        assert!(err.to_string().contains("isn't implemented yet"));
        assert!(calls.borrow().is_empty());
    }
}

#[test]
fn guard_destructive_refuses_production_without_force() {
    let err = guard_destructive("do a dangerous thing", true, false, true).unwrap_err();
    assert!(err.to_string().contains("NODE_ENV=production"));
}

#[test]
fn guard_destructive_allows_production_with_force() {
    guard_destructive("do a dangerous thing", true, true, true).unwrap();
}

#[test]
fn guard_destructive_skips_prompt_when_yes_is_set() {
    // If this didn't skip the confirmation prompt, it would block on stdin.
    guard_destructive("do a dangerous thing", true, false, false).unwrap();
}
