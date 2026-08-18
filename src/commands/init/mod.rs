mod args;
mod config;
mod project_config;
mod templates;

use std::path::PathBuf;

use anyhow::{Context as _, Result};

pub use args::Args;

use crate::shared::context::Context;
use crate::shared::ui;

pub fn run(args: &Args, ctx: &Context) -> Result<()> {
    let name = args
        .name
        .as_deref()
        .context("`name` is required (pass it as an argument, or include it in --json)")?;
    let root = PathBuf::from(name);

    ui::step(&format!(
        "Scaffolding a NestJS project in {} ({}, {})",
        root.display(),
        args.orm.as_str(),
        args.package_manager.command(),
    ));
    for (path, contents) in templates::starter_files(name, args.orm, args.driver)? {
        ctx.fs.write_file(&root.join(path), &contents)?;
    }

    let project_config = project_config::render(args.orm, args.driver, args.package_manager)?;
    ctx.fs
        .write_file(&root.join("ci/config.json"), &project_config)?;

    if !args.skip_git {
        ui::step("Running git init");
        ctx.commands.run("git", &["init"], &root)?;
    }
    if !args.skip_install {
        ui::step(&format!(
            "Installing dependencies with {}",
            args.package_manager.command()
        ));
        ctx.commands
            .run(args.package_manager.command(), &["install"], &root)?;
    }

    ui::success(&format!("Created NestJS project in {}", root.display()));
    Ok(())
}

#[cfg(test)]
mod tests;
