mod args;
mod config;
mod listeners;
mod project_config;
mod templates;

use std::path::PathBuf;

use anyhow::{Context as _, Result};

pub use args::Args;

use crate::shared::context::Context;

pub fn run(args: &Args, ctx: &Context) -> Result<()> {
    listeners::bus(ctx).run("init", |events| {
        let name = args
            .name
            .as_deref()
            .context("`name` is required (pass it as an argument, or include it in --json)")?;
        let root = PathBuf::from(name);

        events.updated(format!(
            "Scaffolding a NestJS project in {} ({}, {})",
            root.display(),
            args.orm.as_str(),
            args.package_manager.command(),
        ));
        for (path, contents) in templates::starter_files(name, args.orm, args.driver)? {
            ctx.fs.write_file(&root.join(path), &contents)?;
        }

        let project_config =
            project_config::render(args.orm, args.driver, args.package_manager)?;
        ctx.fs
            .write_file(&root.join("ci/config.json"), &project_config)?;

        if !args.skip_git {
            events.updated("Running git init");
            ctx.commands.run("git", &["init"], &root)?;
        }
        if !args.skip_install {
            events.updated(format!(
                "Installing dependencies with {}",
                args.package_manager.command()
            ));
            ctx.commands
                .run(args.package_manager.command(), &["install"], &root)?;
        }

        Ok(format!("Created NestJS project in {}", root.display()))
    })
}

#[cfg(test)]
mod tests;
