mod args;
mod config;
mod templates;

use std::path::PathBuf;

use anyhow::{Context as _, Result};

pub use args::Args;

use crate::context::Context;

pub fn run(args: &Args, ctx: &Context) -> Result<()> {
    let name = args
        .name
        .as_deref()
        .context("`name` is required (pass it as an argument, or include it in --json)")?;
    let root = PathBuf::from(name);

    for (path, contents) in templates::starter_files(name)? {
        ctx.fs.write_file(&root.join(path), &contents)?;
    }

    if !args.skip_git {
        ctx.commands.run("git", &["init"], &root)?;
    }
    if !args.skip_install {
        ctx.commands
            .run(args.package_manager.command(), &["install"], &root)?;
    }

    println!("Created NestJS project in {}", root.display());
    Ok(())
}

#[cfg(test)]
mod tests;
