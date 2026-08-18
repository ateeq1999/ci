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

/// `run()` resolves paths against the real `std::env::current_dir()` (it
/// has no `--path` flag, matching `ci db`'s "operate on the project you're
/// standing in" design), so `InMemoryFileSystem` entries need keys that
/// match that same absolute root, not bare relative paths.
fn root() -> std::path::PathBuf {
    std::env::current_dir().unwrap()
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
fn migrate_fresh_drizzle_drops_schema_then_regenerates() {
    let (ctx, calls) = ctx_with_database_url("postgres://user:pass@localhost:5432/my_api");
    run(
        &Args {
            command: Command::Migrate(migrate_args(
                DbOrm::Drizzle,
                Some(MigrateAction::Fresh(destructive())),
            )),
        },
        &ctx,
    )
    .unwrap();

    let calls = calls.borrow();
    assert_eq!(calls.len(), 3);
    assert!(calls[0].starts_with("node -e"));
    assert!(calls[0].contains("require(\"pg\")"), "should use the pg driver by default");
    assert!(calls[0].contains("postgres://user:pass@localhost:5432/my_api"));
    assert!(calls[0].contains("DROP SCHEMA IF EXISTS public CASCADE"));
    assert!(calls[0].contains("DROP SCHEMA IF EXISTS drizzle CASCADE"));
    assert_eq!(calls[1], "npx drizzle-kit generate");
    assert_eq!(calls[2], "npx drizzle-kit migrate");
}

#[test]
fn migrate_fresh_drizzle_postgres_js_driver_uses_the_postgres_js_script() {
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
    };

    run(
        &Args {
            command: Command::Migrate(MigrateArgs {
                action: Some(MigrateAction::Fresh(destructive())),
                orm: None,
            }),
        },
        &ctx,
    )
    .unwrap();

    assert!(calls.borrow()[0].contains("require(\"postgres\")"));
}

#[test]
fn migrate_fresh_drizzle_errors_clearly_without_a_env_file() {
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

    assert!(err.to_string().contains(".env"));
    assert!(err.to_string().contains("not found"));
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
fn migrate_refresh_typeorm_is_not_implemented() {
    let (ctx, _calls) = ctx();
    let err = run(
        &Args {
            command: Command::Migrate(migrate_args(
                DbOrm::Typeorm,
                Some(MigrateAction::Refresh(destructive())),
            )),
        },
        &ctx,
    )
    .unwrap_err();
    assert!(!err.to_string().is_empty());
}

#[test]
fn migrate_refresh_drizzle_reuses_the_fresh_drop_and_rebuild() {
    let (ctx, calls) = ctx_with_database_url("postgres://localhost/my_api");
    run(
        &Args {
            command: Command::Migrate(migrate_args(
                DbOrm::Drizzle,
                Some(MigrateAction::Refresh(destructive())),
            )),
        },
        &ctx,
    )
    .unwrap();

    let calls = calls.borrow();
    assert_eq!(calls.len(), 3);
    assert!(calls[0].starts_with("node -e"));
    assert_eq!(calls[1], "npx drizzle-kit generate");
    assert_eq!(calls[2], "npx drizzle-kit migrate");
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
