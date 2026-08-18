use std::path::PathBuf;

/// (relative path, template contents) pairs for the minimal NestJS starter.
const FILES: &[(&str, &str)] = &[
    (
        "package.json",
        include_str!("../../templates/nestjs-starter/package.json"),
    ),
    (
        "tsconfig.json",
        include_str!("../../templates/nestjs-starter/tsconfig.json"),
    ),
    (
        "tsconfig.build.json",
        include_str!("../../templates/nestjs-starter/tsconfig.build.json"),
    ),
    (
        "nest-cli.json",
        include_str!("../../templates/nestjs-starter/nest-cli.json"),
    ),
    (
        "src/main.ts",
        include_str!("../../templates/nestjs-starter/src/main.ts"),
    ),
    (
        "src/app.module.ts",
        include_str!("../../templates/nestjs-starter/src/app.module.ts"),
    ),
    (
        "src/app.controller.ts",
        include_str!("../../templates/nestjs-starter/src/app.controller.ts"),
    ),
    (
        "src/app.service.ts",
        include_str!("../../templates/nestjs-starter/src/app.service.ts"),
    ),
    (
        "src/app.controller.spec.ts",
        include_str!("../../templates/nestjs-starter/src/app.controller.spec.ts"),
    ),
];

/// Returns the starter project's files with `{{project_name}}` substituted.
pub fn starter_files(project_name: &str) -> Vec<(PathBuf, String)> {
    FILES
        .iter()
        .map(|(path, contents)| {
            (
                PathBuf::from(path),
                contents.replace("{{project_name}}", project_name),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_project_name_into_package_json() {
        let files = starter_files("my-api");
        let (_, package_json) = files
            .iter()
            .find(|(path, _)| path == std::path::Path::new("package.json"))
            .expect("package.json should be present");

        assert!(package_json.contains("\"name\": \"my-api\""));
        assert!(!package_json.contains("{{project_name}}"));
    }

    #[test]
    fn includes_every_expected_file() {
        let files = starter_files("my-api");
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
}
