use std::path::PathBuf;

use super::config;

/// (relative path, template contents) pairs for the minimal NestJS starter.
/// The actual content lives under the fixed root `templates/init/` folder —
/// this is only the manifest of which files to render and how.
const FILES: &[(&str, &str)] = &[
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
];

/// Returns the starter project's files with substitution placeholders
/// (`{{project_name}}`, `{{package_version}}`, ...) filled in.
pub fn starter_files(project_name: &str) -> Vec<(PathBuf, String)> {
    FILES
        .iter()
        .map(|(path, contents)| {
            let rendered = contents
                .replace("{{project_name}}", project_name)
                .replace("{{package_version}}", config::STARTER_PACKAGE_VERSION)
                .replace("{{node_engine_range}}", config::NODE_ENGINE_RANGE)
                .replace("{{nest_cli_schema_url}}", config::NEST_CLI_SCHEMA_URL);
            (PathBuf::from(path), rendered)
        })
        .collect()
}

