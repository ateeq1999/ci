use std::path::{Path, PathBuf};

use super::args::{DbOrm, PackageManager};
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
        orm: DbOrm::Drizzle,
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
        orm: DbOrm::Drizzle,
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
        orm: DbOrm::Drizzle,
        skip_install: false,
        skip_git: false,
    };

    run(&args, &ctx).unwrap();

    let calls = calls.borrow();
    assert_eq!(calls.as_slice(), ["git init", "pnpm install"]);
}

#[test]
fn substitutes_all_placeholders_into_package_json() {
    let files = templates::starter_files("my-api", DbOrm::Drizzle).unwrap();
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
    assert!(!package_json.contains("{%"));
}

#[test]
fn substitutes_schema_url_into_nest_cli_json() {
    let files = templates::starter_files("my-api", DbOrm::Drizzle).unwrap();
    let (_, nest_cli_json) = files
        .iter()
        .find(|(path, _)| path == Path::new("nest-cli.json"))
        .expect("nest-cli.json should be present");

    assert!(nest_cli_json.contains(config::NEST_CLI_SCHEMA_URL));
}

#[test]
fn includes_every_expected_file() {
    let files = templates::starter_files("my-api", DbOrm::Drizzle).unwrap();
    let paths: Vec<_> = files.iter().map(|(p, _)| p.clone()).collect();

    for expected in [
        ".env.example",
        ".env",
        "package.json",
        "tsconfig.json",
        "tsconfig.build.json",
        "nest-cli.json",
        "src/main.ts",
        "src/app.module.ts",
        "src/app.controller.ts",
        "src/app.service.ts",
        "src/app.controller.spec.ts",
        "src/config/env.validation.ts",
        "src/database/database-type.ts",
        "src/database/database.provider.ts",
        "src/database/database.module.ts",
        "src/database/schema/index.ts",
        "src/database/schema/users.ts",
        "src/database/postgres-client.provider.ts",
        "drizzle.config.ts",
    ] {
        assert!(
            paths.contains(&PathBuf::from(expected)),
            "missing {expected}"
        );
    }
}

#[test]
fn dot_env_mirrors_dot_env_example() {
    let files = templates::starter_files("my-api", DbOrm::Drizzle).unwrap();
    let (_, env_example) = files
        .iter()
        .find(|(path, _)| path == Path::new(".env.example"))
        .expect(".env.example should be present");
    let (_, env) = files
        .iter()
        .find(|(path, _)| path == Path::new(".env"))
        .expect(".env should be present");

    assert_eq!(env, env_example);
    assert!(env.contains("DATABASE_URL="));
    assert!(!env.contains("{{"));
}

#[test]
fn drizzle_is_the_default_orm_and_pulls_in_schema_folder() {
    let files = templates::starter_files("my-api", DbOrm::Drizzle).unwrap();
    let (_, provider) = files
        .iter()
        .find(|(path, _)| path == Path::new("src/database/database.provider.ts"))
        .expect("database.provider.ts should be present");

    assert!(provider.contains("drizzle-orm/postgres-js"));
    assert!(provider.contains("databaseProvider"));
    assert!(provider.contains("POSTGRES_CLIENT_TOKEN"));
    assert!(!provider.contains("{%"));

    let (_, postgres_client) = files
        .iter()
        .find(|(path, _)| path == Path::new("src/database/postgres-client.provider.ts"))
        .expect("postgres-client.provider.ts should be present");
    assert!(postgres_client.contains("onApplicationShutdown"));
    assert!(postgres_client.contains("postgresClientProvider"));

    let (_, module) = files
        .iter()
        .find(|(path, _)| path == Path::new("src/database/database.module.ts"))
        .expect("database.module.ts should be present");
    assert!(module.contains("postgresClientProvider"));
    assert!(!module.contains("{%"));

    let (_, drizzle_config) = files
        .iter()
        .find(|(path, _)| path == Path::new("drizzle.config.ts"))
        .expect("drizzle.config.ts should be present");
    assert!(drizzle_config.contains("dialect: \"postgresql\""));

    let (_, users_schema) = files
        .iter()
        .find(|(path, _)| path == Path::new("src/database/schema/users.ts"))
        .expect("schema/users.ts should be present");
    assert!(users_schema.contains("uuid(\"id\")"));
    assert!(users_schema.contains("email"));
    assert!(users_schema.contains("password"));

    let (_, schema_index) = files
        .iter()
        .find(|(path, _)| path == Path::new("src/database/schema/index.ts"))
        .expect("schema/index.ts should be present");
    assert!(schema_index.contains("./users"));

    let (_, package_json) = files
        .iter()
        .find(|(path, _)| path == Path::new("package.json"))
        .expect("package.json should be present");
    assert!(package_json.contains("\"postgres\":"));
    assert!(!package_json.contains("\"pg\":"));
    assert!(package_json.contains("\"db:generate\": \"drizzle-kit generate\""));
    assert!(!package_json.contains("{%"));

    assert!(
        !files
            .iter()
            .any(|(path, _)| path == Path::new("prisma/schema.prisma")),
        "drizzle should not include prisma/schema.prisma"
    );
}

#[test]
fn prisma_gets_a_schema_prisma_instead_of_drizzle_schema() {
    let files = templates::starter_files("my-api", DbOrm::Prisma).unwrap();

    let (_, schema_prisma) = files
        .iter()
        .find(|(path, _)| path == Path::new("prisma/schema.prisma"))
        .expect("prisma/schema.prisma should be present");
    assert!(schema_prisma.contains("model User"));
    assert!(!schema_prisma.contains("{%"));

    assert!(
        !files
            .iter()
            .any(|(path, _)| path == Path::new("src/database/schema/index.ts")),
        "prisma should not include drizzle's schema folder"
    );
}

#[test]
fn typeorm_gets_neither_drizzle_schema_nor_prisma_schema() {
    let files = templates::starter_files("my-api", DbOrm::Typeorm).unwrap();

    assert!(
        !files
            .iter()
            .any(|(path, _)| path == Path::new("src/database/schema/index.ts"))
    );
    assert!(
        !files
            .iter()
            .any(|(path, _)| path == Path::new("prisma/schema.prisma"))
    );

    let (_, package_json) = files
        .iter()
        .find(|(path, _)| path == Path::new("package.json"))
        .expect("package.json should be present");
    assert!(!package_json.contains("{%"));
}
