use std::path::{Path, PathBuf};

use super::args::PackageManager;
use super::*;
use crate::context::NoopCommandRunner;
use crate::fs::InMemoryFileSystem;

#[test]
fn writes_starter_files_under_project_dir() {
    let fs = InMemoryFileSystem::default();
    let written = fs.written.clone();
    let ctx = Context {
        fs: Box::new(fs),
        commands: Box::new(NoopCommandRunner::default()),
    };
    let args = Args {
        name: Some("my-api".into()),
        package_manager: PackageManager::Npm,
        skip_install: true,
        skip_git: true,
    };

    run(&args, &ctx).unwrap();

    let written = written.borrow();
    assert!(written.contains_key(Path::new("my-api/package.json")));
    assert!(written.contains_key(Path::new("my-api/src/main.ts")));
}

#[test]
fn errors_when_name_missing() {
    let ctx = Context {
        fs: Box::new(InMemoryFileSystem::default()),
        commands: Box::new(NoopCommandRunner::default()),
    };
    let args = Args::default();

    let err = run(&args, &ctx).unwrap_err();
    assert!(err.to_string().contains("`name` is required"));
}

#[test]
fn skips_git_and_install_when_requested() {
    let commands = NoopCommandRunner::default();
    let calls = commands.calls.clone();
    let ctx = Context {
        fs: Box::new(InMemoryFileSystem::default()),
        commands: Box::new(commands),
    };
    let args = Args {
        name: Some("my-api".into()),
        package_manager: PackageManager::Npm,
        skip_install: true,
        skip_git: true,
    };

    run(&args, &ctx).unwrap();

    assert!(calls.borrow().is_empty());
}

#[test]
fn runs_git_and_package_manager_install_by_default() {
    let commands = NoopCommandRunner::default();
    let calls = commands.calls.clone();
    let ctx = Context {
        fs: Box::new(InMemoryFileSystem::default()),
        commands: Box::new(commands),
    };
    let args = Args {
        name: Some("my-api".into()),
        package_manager: PackageManager::Pnpm,
        skip_install: false,
        skip_git: false,
    };

    run(&args, &ctx).unwrap();

    let calls = calls.borrow();
    assert_eq!(calls.as_slice(), ["git init", "pnpm install"]);
}

#[test]
fn substitutes_all_placeholders_into_package_json() {
    let files = templates::starter_files("my-api");
    let (_, package_json) = files
        .iter()
        .find(|(path, _)| path == Path::new("package.json"))
        .expect("package.json should be present");

    assert!(package_json.contains("\"name\": \"my-api\""));
    assert!(package_json.contains(&format!(
        "\"version\": \"{}\"",
        config::STARTER_PACKAGE_VERSION
    )));
    assert!(package_json.contains(&format!("\"node\": \"{}\"", config::NODE_ENGINE_RANGE)));
    assert!(!package_json.contains("{{"));
}

#[test]
fn substitutes_schema_url_into_nest_cli_json() {
    let files = templates::starter_files("my-api");
    let (_, nest_cli_json) = files
        .iter()
        .find(|(path, _)| path == Path::new("nest-cli.json"))
        .expect("nest-cli.json should be present");

    assert!(nest_cli_json.contains(config::NEST_CLI_SCHEMA_URL));
}

#[test]
fn includes_every_expected_file() {
    let files = templates::starter_files("my-api");
    let paths: Vec<_> = files.iter().map(|(p, _)| p.clone()).collect();

    for expected in [
        "package.json",
        "tsconfig.json",
        "tsconfig.build.json",
        "nest-cli.json",
        "src/main.ts",
        "src/app.module.ts",
        "src/app.controller.ts",
        "src/app.service.ts",
        "src/app.controller.spec.ts",
    ] {
        assert!(
            paths.contains(&PathBuf::from(expected)),
            "missing {expected}"
        );
    }
}
