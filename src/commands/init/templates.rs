use std::path::PathBuf;

use anyhow::Result;
use atom_engine::Atom;
use serde_json::json;

use super::args::{DbOrm, DrizzleDriver};
use super::config;

/// (relative path, template contents) pairs for the minimal NestJS starter.
/// The actual content lives under the fixed root `templates/init/` folder —
/// this is only the manifest of which files to render and how.
const FILES: &[(&str, &str)] = &[
    (
        ".gitignore",
        include_str!("../../../templates/init/.gitignore"),
    ),
    (
        ".env.example",
        include_str!("../../../templates/init/.env.example"),
    ),
    (
        "package.json",
        include_str!("../../../templates/init/package.json"),
    ),
    (
        "tsconfig.json",
        include_str!("../../../templates/init/tsconfig.json"),
    ),
    (
        "tsconfig.build.json",
        include_str!("../../../templates/init/tsconfig.build.json"),
    ),
    (
        "nest-cli.json",
        include_str!("../../../templates/init/nest-cli.json"),
    ),
    (
        "src/main.ts",
        include_str!("../../../templates/init/src/main.ts"),
    ),
    (
        "src/app.module.ts",
        include_str!("../../../templates/init/src/app.module.ts"),
    ),
    (
        "src/app.controller.ts",
        include_str!("../../../templates/init/src/app.controller.ts"),
    ),
    (
        "src/app.service.ts",
        include_str!("../../../templates/init/src/app.service.ts"),
    ),
    (
        "src/app.controller.spec.ts",
        include_str!("../../../templates/init/src/app.controller.spec.ts"),
    ),
    (
        "src/config/env.validation.ts",
        include_str!("../../../templates/init/src/config/env.validation.ts"),
    ),
    (
        "src/logger/logger.service.ts",
        include_str!("../../../templates/add/logger/logger.service.ts"),
    ),
    (
        "src/logger/logger.module.ts",
        include_str!("../../../templates/add/logger/logger.module.ts"),
    ),
    (
        "src/database/database.types.ts",
        include_str!("../../../templates/db/database.types.ts"),
    ),
    (
        "src/database/database.constants.ts",
        include_str!("../../../templates/db/database.constants.ts"),
    ),
    (
        "src/database/database.provider.ts",
        include_str!("../../../templates/db/database.provider.ts"),
    ),
    (
        "src/database/database.module.ts",
        include_str!("../../../templates/db/database.module.ts"),
    ),
];

/// ORM-specific files layered on top of `FILES` — only rendered when the
/// chosen `db_orm` needs them.
const DRIZZLE_FILES: &[(&str, &str)] = &[
    (
        "src/database/schema/index.ts",
        include_str!("../../../templates/db/schema/index.ts"),
    ),
    (
        "src/database/schema/users.ts",
        include_str!("../../../templates/db/schema/users.ts"),
    ),
    (
        "src/database/database-client.provider.ts",
        include_str!("../../../templates/db/database-client.provider.ts"),
    ),
    (
        "drizzle.config.ts",
        include_str!("../../../templates/db/drizzle.config.ts"),
    ),
];

const PRISMA_FILES: &[(&str, &str)] = &[(
    "prisma/schema.prisma",
    include_str!("../../../templates/db/schema.prisma"),
)];

const TYPEORM_FILES: &[(&str, &str)] = &[(
    "src/database/data-source.ts",
    include_str!("../../../templates/db/data-source.ts"),
)];

/// Returns the starter project's files with substitution placeholders
/// (`{{project_name}}`, `{{package_version}}`, ...) filled in, plus a
/// `DatabaseModule` wired up for `db_orm`.
pub fn starter_files(
    project_name: &str,
    db_orm: DbOrm,
    db_driver: DrizzleDriver,
) -> Result<Vec<(PathBuf, String)>> {
    let mut engine = Atom::new();
    let ctx = json!({
        "project_name": project_name,
        "package_version": config::STARTER_PACKAGE_VERSION,
        "node_engine_range": config::NODE_ENGINE_RANGE,
        "nest_cli_schema_url": config::NEST_CLI_SCHEMA_URL,
        "db_orm": db_orm.as_str(),
        "db_driver": db_driver.as_str(),
    });

    let orm_files: &[(&str, &str)] = match db_orm {
        DbOrm::Drizzle => DRIZZLE_FILES,
        DbOrm::Prisma => PRISMA_FILES,
        DbOrm::Typeorm => TYPEORM_FILES,
    };

    let mut files: Vec<(PathBuf, String)> = FILES
        .iter()
        .chain(orm_files)
        .map(|(path, contents)| {
            engine
                .add_template(path, contents)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let rendered = engine
                .render(path, &ctx)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok((PathBuf::from(*path), rendered))
        })
        .collect::<Result<_>>()?;

    // `cp .env.example .env` — the generated project should already have a
    // working local `.env` (gitignored) instead of requiring that manual step.
    let env_example = files
        .iter()
        .find(|(path, _)| path == std::path::Path::new(".env.example"))
        .map(|(_, contents)| contents.clone())
        .expect(".env.example must be in FILES");
    files.push((PathBuf::from(".env"), env_example));

    Ok(files)
}

