use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::args::InitArgs;
use crate::fs::FileSystem;
use crate::templates::nestjs;

pub fn run(args: &InitArgs, fs: &dyn FileSystem) -> Result<()> {
    let name = args
        .name
        .as_deref()
        .context("`name` is required (pass it as an argument, or include it in --json)")?;
    let root = PathBuf::from(name);

    for (path, contents) in nestjs::starter_files(name) {
        fs.write_file(&root.join(path), &contents)?;
    }

    if !args.skip_git {
        run_command("git", &["init"], &root)?;
    }
    if !args.skip_install {
        run_command(args.package_manager.command(), &["install"], &root)?;
    }

    println!("Created NestJS project in {}", root.display());
    Ok(())
}

fn run_command(program: &str, args: &[&str], cwd: &Path) -> Result<()> {
    let status = std::process::Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("failed to run `{program}`"))?;

    if !status.success() {
        bail!("`{program} {}` exited with {status}", args.join(" "));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::PackageManager;
    use crate::fs::InMemoryFileSystem;

    #[test]
    fn writes_starter_files_under_project_dir() {
        let fs = InMemoryFileSystem::default();
        let args = InitArgs {
            name: Some("my-api".into()),
            package_manager: PackageManager::Npm,
            skip_install: true,
            skip_git: true,
        };

        run(&args, &fs).unwrap();

        let written = fs.written.borrow();
        assert!(written.contains_key(Path::new("my-api/package.json")));
        assert!(written.contains_key(Path::new("my-api/src/main.ts")));
    }

    #[test]
    fn errors_when_name_missing() {
        let fs = InMemoryFileSystem::default();
        let args = InitArgs::default();

        let err = run(&args, &fs).unwrap_err();
        assert!(err.to_string().contains("`name` is required"));
    }
}
