mod args;
mod config;
mod listeners;
mod project_config;
// `pub(crate)` (not private) so `commands::add`'s tests can seed fixtures
// from real `init` output instead of hand-typed copies that can drift.
pub(crate) mod templates;

use std::path::PathBuf;

use anyhow::{Context as _, Result};

pub use args::Args;

use crate::shared::context::Context;

pub fn run(args: &Args, ctx: &Context) -> Result<()> {
    // Validated and computed *before* the event lifecycle starts (unlike
    // every other step below, which runs inside `bus.run`'s closure) so
    // `listeners::bus` has a project root to attach history to even when
    // nothing else about this run succeeds. If `name` itself is missing,
    // there's no project to record history against at all — that failure
    // is reported the same way it always was, just without ever
    // constructing a bus for it.
    let name = args
        .name
        .as_deref()
        .context("`name` is required (pass it as an argument, or include it in --json)")?;
    let root = PathBuf::from(name);

    listeners::bus(ctx, &root).run("init", |events| {
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
