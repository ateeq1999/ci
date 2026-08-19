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

fn written(ctx: &Context, path: &str) -> String {
    ctx.fs
        .try_read_to_string(Path::new(path))
        .unwrap()
        .unwrap()
}

#[test]
fn add_dependencies_inserts_new_entries() {
    let ctx = ctx_with_file(
        "proj/package.json",
        r#"{"dependencies":{"@nestjs/common":"^10.0.0"}}"#,
    );

    add_dependencies(
        &ctx,
        Path::new("proj"),
        &[("class-validator", "^0.14.1")],
    )
    .unwrap();

    let out = written(&ctx, "proj/package.json");
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["dependencies"]["@nestjs/common"], "^10.0.0");
    assert_eq!(parsed["dependencies"]["class-validator"], "^0.14.1");
}

#[test]
fn add_dependencies_does_not_overwrite_an_existing_version() {
    let ctx = ctx_with_file(
        "proj/package.json",
        r#"{"dependencies":{"class-validator":"^0.13.0"}}"#,
    );

    add_dependencies(&ctx, Path::new("proj"), &[("class-validator", "^0.14.1")]).unwrap();

    let out = written(&ctx, "proj/package.json");
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["dependencies"]["class-validator"], "^0.13.0");
}

#[test]
fn add_dependencies_errors_clearly_when_package_json_missing() {
    let ctx = ctx_with_file("proj/.gitignore", "node_modules\n");
    let err = add_dependencies(&ctx, Path::new("proj"), &[("x", "^1.0.0")]).unwrap_err();
    assert!(err.to_string().contains("not found"));
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
