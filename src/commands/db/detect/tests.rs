use std::path::{Path, PathBuf};

use super::*;
use crate::shared::context::NoopCommandRunner;
use crate::shared::fs::InMemoryFileSystem;

fn ctx_with_files(files: &[(&str, &str)]) -> Context {
    let fs = InMemoryFileSystem::default();
    for (path, contents) in files {
        fs.written
            .borrow_mut()
            .insert(PathBuf::from(path), contents.to_string());
    }
    Context {
        fs: Box::new(fs),
        commands: Box::new(NoopCommandRunner::default()),
    }
}

#[test]
fn reads_orm_and_driver_from_config_json() {
    let ctx = ctx_with_files(&[(
        "proj/ci/config.json",
        r#"{"ciVersion":"0.1.1","orm":"drizzle","driver":"postgres-js","packageManager":"npm"}"#,
    )]);

    let detected = detect(&ctx, Path::new("proj"), None).unwrap();
    assert_eq!(detected.orm, DbOrm::Drizzle);
    assert_eq!(detected.driver, DrizzleDriver::PostgresJs);
}

#[test]
fn config_json_without_driver_field_defaults_driver() {
    let ctx = ctx_with_files(&[(
        "proj/ci/config.json",
        r#"{"ciVersion":"0.1.1","orm":"prisma","packageManager":"npm"}"#,
    )]);

    let detected = detect(&ctx, Path::new("proj"), None).unwrap();
    assert_eq!(detected.orm, DbOrm::Prisma);
}

#[test]
fn orm_override_skips_detection_entirely() {
    let ctx = ctx_with_files(&[]);

    let detected = detect(&ctx, Path::new("proj"), Some(DbOrm::Typeorm)).unwrap();
    assert_eq!(detected.orm, DbOrm::Typeorm);
}

#[test]
fn falls_back_to_marker_file_when_no_config_json() {
    let ctx = ctx_with_files(&[("proj/prisma/schema.prisma", "// schema")]);

    let detected = detect(&ctx, Path::new("proj"), None).unwrap();
    assert_eq!(detected.orm, DbOrm::Prisma);
}

#[test]
fn errors_when_nothing_found() {
    let ctx = ctx_with_files(&[]);

    let err = detect(&ctx, Path::new("proj"), None).unwrap_err();
    assert!(err.to_string().contains("couldn't detect an ORM"));
}

#[test]
fn errors_when_multiple_markers_found() {
    let ctx = ctx_with_files(&[
        ("proj/drizzle.config.ts", "// config"),
        ("proj/prisma/schema.prisma", "// schema"),
    ]);

    let err = detect(&ctx, Path::new("proj"), None).unwrap_err();
    assert!(err.to_string().contains("ambiguous ORM"));
}

#[test]
fn config_json_takes_priority_over_markers() {
    let ctx = ctx_with_files(&[
        (
            "proj/ci/config.json",
            r#"{"ciVersion":"0.1.1","orm":"typeorm","packageManager":"npm"}"#,
        ),
        ("proj/prisma/schema.prisma", "// schema"),
    ]);

    let detected = detect(&ctx, Path::new("proj"), None).unwrap();
    assert_eq!(detected.orm, DbOrm::Typeorm);
}
