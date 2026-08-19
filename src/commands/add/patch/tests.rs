use std::path::{Path, PathBuf};

use super::*;
use crate::shared::context::NoopCommandRunner;
use crate::shared::fs::InMemoryFileSystem;
use crate::shared::ui::ConsoleUi;

fn ctx_with_file(path: &str, contents: &str) -> Context {
    let fs = InMemoryFileSystem::default();
    fs.written
        .borrow_mut()
        .insert(PathBuf::from(path), contents.to_string());
    Context {
        fs: Box::new(fs),
        commands: Box::new(NoopCommandRunner::default()),
        ui: Box::new(ConsoleUi),
    }
}

fn ctx_with_files(files: &[(&str, &str)]) -> (Context, std::rc::Rc<std::cell::RefCell<Vec<String>>>) {
    let fs = InMemoryFileSystem::default();
    for (path, contents) in files {
        fs.written
            .borrow_mut()
            .insert(PathBuf::from(*path), contents.to_string());
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

fn written(ctx: &Context, path: &str) -> String {
    ctx.fs
        .try_read_to_string(Path::new(path))
        .unwrap()
        .unwrap()
}

#[test]
fn detect_package_manager_reads_ci_config_json() {
    let ctx = ctx_with_file(
        "proj/ci/config.json",
        r#"{"ciVersion":"0.1.2","orm":"drizzle","packageManager":"pnpm"}"#,
    );
    assert_eq!(
        detect_package_manager(&ctx, Path::new("proj")),
        PackageManager::Pnpm
    );
}

#[test]
fn detect_package_manager_falls_back_to_npm_when_config_missing() {
    let ctx = ctx_with_file("proj/.gitignore", "node_modules\n");
    assert_eq!(
        detect_package_manager(&ctx, Path::new("proj")),
        PackageManager::Npm
    );
}

#[test]
fn detect_package_manager_falls_back_to_npm_when_config_unparseable() {
    let ctx = ctx_with_file("proj/ci/config.json", "not json");
    assert_eq!(
        detect_package_manager(&ctx, Path::new("proj")),
        PackageManager::Npm
    );
}

#[test]
fn install_dependencies_uses_install_verb_for_npm() {
    let (ctx, calls) = ctx_with_files(&[]);
    install_dependencies(&ctx, Path::new("proj"), PackageManager::Npm, &["foo", "bar"]).unwrap();
    assert_eq!(calls.borrow().as_slice(), ["npm install foo bar"]);
}

#[test]
fn install_dependencies_uses_add_verb_for_pnpm_and_yarn() {
    for pm in [PackageManager::Pnpm, PackageManager::Yarn] {
        let (ctx, calls) = ctx_with_files(&[]);
        install_dependencies(&ctx, Path::new("proj"), pm, &["foo"]).unwrap();
        assert_eq!(
            calls.borrow().as_slice(),
            [format!("{} add foo", pm.command())]
        );
    }
}

#[test]
fn insert_after_inserts_once_and_is_idempotent() {
    let ctx = ctx_with_file("proj/main.ts", "import { NestFactory } from 'x';\nbootstrap();\n");

    let first = insert_after(
        &ctx,
        Path::new("proj/main.ts"),
        "import { NestFactory }",
        "import { Marker }",
        "import { Marker } from 'y';",
    )
    .unwrap();
    assert!(first);
    assert_eq!(
        written(&ctx, "proj/main.ts"),
        "import { NestFactory } from 'x';\nimport { Marker } from 'y';\nbootstrap();\n"
    );

    let second = insert_after(
        &ctx,
        Path::new("proj/main.ts"),
        "import { NestFactory }",
        "import { Marker }",
        "import { Marker } from 'y';",
    )
    .unwrap();
    assert!(!second, "second call should be a no-op");
    // Content unchanged — no duplicate insertion.
    assert_eq!(
        written(&ctx, "proj/main.ts"),
        "import { NestFactory } from 'x';\nimport { Marker } from 'y';\nbootstrap();\n"
    );
}

#[test]
fn insert_after_errors_when_anchor_missing() {
    let ctx = ctx_with_file("proj/main.ts", "console.log('hi');\n");
    let err = insert_after(
        &ctx,
        Path::new("proj/main.ts"),
        "not there",
        "marker",
        "line",
    )
    .unwrap_err();
    assert!(err.to_string().contains("hand-edited"));
}

#[test]
fn insert_after_last_import_lands_after_the_last_import_line() {
    let ctx = ctx_with_file(
        "proj/app.module.ts",
        "import { Module } from '@nestjs/common';\nimport { DatabaseModule } from './database/database.module';\n\n@Module({})\n",
    );

    let inserted = insert_after_last_import(
        &ctx,
        Path::new("proj/app.module.ts"),
        "import { CacheModule }",
        "import { CacheModule } from '@nestjs/cache-manager';",
    )
    .unwrap();
    assert!(inserted);
    assert_eq!(
        written(&ctx, "proj/app.module.ts"),
        "import { Module } from '@nestjs/common';\nimport { DatabaseModule } from './database/database.module';\nimport { CacheModule } from '@nestjs/cache-manager';\n\n@Module({})\n"
    );
}

#[test]
fn insert_after_last_import_stacks_after_a_previously_inserted_import() {
    // Simulates `ci add cache` followed by `ci add schedule`: the second
    // subcommand's import must land after the first's, not both landing
    // in the same fixed spot.
    let ctx = ctx_with_file(
        "proj/app.module.ts",
        "import { Module } from '@nestjs/common';\nimport { DatabaseModule } from './database/database.module';\n\n@Module({})\n",
    );

    insert_after_last_import(
        &ctx,
        Path::new("proj/app.module.ts"),
        "import { CacheModule }",
        "import { CacheModule } from '@nestjs/cache-manager';",
    )
    .unwrap();
    insert_after_last_import(
        &ctx,
        Path::new("proj/app.module.ts"),
        "import { ScheduleModule }",
        "import { ScheduleModule } from '@nestjs/schedule';",
    )
    .unwrap();

    assert_eq!(
        written(&ctx, "proj/app.module.ts"),
        "import { Module } from '@nestjs/common';\nimport { DatabaseModule } from './database/database.module';\nimport { CacheModule } from '@nestjs/cache-manager';\nimport { ScheduleModule } from '@nestjs/schedule';\n\n@Module({})\n"
    );
}

#[test]
fn insert_after_last_import_is_idempotent() {
    let ctx = ctx_with_file(
        "proj/app.module.ts",
        "import { Module } from '@nestjs/common';\n\n@Module({})\n",
    );

    let first = insert_after_last_import(
        &ctx,
        Path::new("proj/app.module.ts"),
        "import { CacheModule }",
        "import { CacheModule } from '@nestjs/cache-manager';",
    )
    .unwrap();
    assert!(first);

    let second = insert_after_last_import(
        &ctx,
        Path::new("proj/app.module.ts"),
        "import { CacheModule }",
        "import { CacheModule } from '@nestjs/cache-manager';",
    )
    .unwrap();
    assert!(!second, "second call should be a no-op");
}

#[test]
fn insert_after_last_import_errors_when_no_import_found() {
    let ctx = ctx_with_file("proj/app.module.ts", "@Module({})\n");
    let err = insert_after_last_import(
        &ctx,
        Path::new("proj/app.module.ts"),
        "marker",
        "import { X } from 'y';",
    )
    .unwrap_err();
    assert!(err.to_string().contains("no `import` line found"));
}

#[test]
fn insert_before_lands_right_above_the_anchor_and_is_idempotent() {
    let ctx = ctx_with_file(
        "proj/main.ts",
        "const app = await NestFactory.create(AppModule);\nawait app.listen(3000);\n",
    );

    let first = insert_before(
        &ctx,
        Path::new("proj/main.ts"),
        "await app.listen(",
        "app.useLogger",
        "app.useLogger(app.get(AppLogger));",
    )
    .unwrap();
    assert!(first);
    assert_eq!(
        written(&ctx, "proj/main.ts"),
        "const app = await NestFactory.create(AppModule);\napp.useLogger(app.get(AppLogger));\nawait app.listen(3000);\n"
    );

    let second = insert_before(
        &ctx,
        Path::new("proj/main.ts"),
        "await app.listen(",
        "app.useLogger",
        "app.useLogger(app.get(AppLogger));",
    )
    .unwrap();
    assert!(!second, "second call should be a no-op");
}

#[test]
fn insert_before_stacks_after_content_inserted_by_an_earlier_insert_after_call() {
    // Simulates `ci add validation` (inserts right after NestFactory.create
    // via insert_after) followed by `ci add logger` (inserts right before
    // app.listen via insert_before) — the two anchors are structurally
    // distinct, so there's no collision regardless of run order.
    let ctx = ctx_with_file(
        "proj/main.ts",
        "const app = await NestFactory.create(AppModule);\nawait app.listen(3000);\n",
    );

    insert_after(
        &ctx,
        Path::new("proj/main.ts"),
        "const app = await NestFactory.create(AppModule);",
        "app.useGlobalPipes",
        "app.useGlobalPipes(new ValidationPipe());",
    )
    .unwrap();
    insert_before(
        &ctx,
        Path::new("proj/main.ts"),
        "await app.listen(",
        "app.useLogger",
        "app.useLogger(app.get(AppLogger));",
    )
    .unwrap();

    assert_eq!(
        written(&ctx, "proj/main.ts"),
        "const app = await NestFactory.create(AppModule);\napp.useGlobalPipes(new ValidationPipe());\napp.useLogger(app.get(AppLogger));\nawait app.listen(3000);\n"
    );
}

#[test]
fn insert_before_errors_when_anchor_missing() {
    let ctx = ctx_with_file("proj/main.ts", "console.log('hi');\n");
    let err = insert_before(
        &ctx,
        Path::new("proj/main.ts"),
        "not there",
        "marker",
        "line",
    )
    .unwrap_err();
    assert!(err.to_string().contains("hand-edited"));
}

#[test]
fn write_file_if_absent_writes_once_and_never_overwrites() {
    let ctx = ctx_with_file("proj/.gitignore", "node_modules\n");

    let first = write_file_if_absent(&ctx, Path::new("proj/logger.service.ts"), "// v1\n").unwrap();
    assert!(first);
    assert_eq!(written(&ctx, "proj/logger.service.ts"), "// v1\n");

    let second =
        write_file_if_absent(&ctx, Path::new("proj/logger.service.ts"), "// v2\n").unwrap();
    assert!(!second, "should not overwrite an already-present file");
    assert_eq!(written(&ctx, "proj/logger.service.ts"), "// v1\n");
}

#[test]
fn insert_into_array_adds_indented_item_once() {
    let ctx = ctx_with_file(
        "proj/app.module.ts",
        "@Module({\n  imports: [\n    Existing,\n  ],\n})\n",
    );

    let first = insert_into_array(
        &ctx,
        Path::new("proj/app.module.ts"),
        "imports: [",
        "NewThing",
        "NewThing,",
    )
    .unwrap();
    assert!(first);
    let out = written(&ctx, "proj/app.module.ts");
    assert!(out.contains("  imports: [\n    NewThing,\n    Existing,\n"));

    let second = insert_into_array(
        &ctx,
        Path::new("proj/app.module.ts"),
        "imports: [",
        "NewThing",
        "NewThing,",
    )
    .unwrap();
    assert!(!second, "second call should be a no-op");
}

#[test]
fn append_line_adds_to_the_end_once() {
    let ctx = ctx_with_file("proj/.env", "NODE_ENV=development\n");

    let first = append_line(&ctx, Path::new("proj/.env"), "REDIS_URL", "REDIS_URL=x").unwrap();
    assert!(first);
    assert_eq!(
        written(&ctx, "proj/.env"),
        "NODE_ENV=development\nREDIS_URL=x\n"
    );

    let second = append_line(&ctx, Path::new("proj/.env"), "REDIS_URL", "REDIS_URL=x").unwrap();
    assert!(!second, "second call should be a no-op");
}
