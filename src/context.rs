use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use anyhow::{bail, Context as _, Result};

use crate::fs::{FileSystem, RealFileSystem};

/// Runs an external command (e.g. `git`, `npm`) in a given directory.
/// Abstracted so commands never call `std::process::Command` directly.
pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str], cwd: &Path) -> Result<()>;
}

pub struct RealCommandRunner;

impl CommandRunner for RealCommandRunner {
    fn run(&self, program: &str, args: &[&str], cwd: &Path) -> Result<()> {
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
}

// `calls` is `Rc<RefCell<..>>` so tests can hold a cloned handle after this
// runner is moved into a `Box<dyn CommandRunner>` and still observe calls
// made through the trait object. Only exercised by #[cfg(test)] code in
// command modules; `cargo build` alone can't see those call sites.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Default)]
pub struct NoopCommandRunner {
    pub calls: Rc<RefCell<Vec<String>>>,
}

impl CommandRunner for NoopCommandRunner {
    fn run(&self, program: &str, args: &[&str], _cwd: &Path) -> Result<()> {
        self.calls
            .borrow_mut()
            .push(format!("{program} {}", args.join(" ")));
        Ok(())
    }
}

/// Dependency container passed into every command's `run()`, so commands
/// depend only on the traits they need instead of reaching for `std::fs`
/// or `std::process::Command` directly.
pub struct Context {
    pub fs: Box<dyn FileSystem>,
    pub commands: Box<dyn CommandRunner>,
}

impl Context {
    pub fn real() -> Self {
        Self {
            fs: Box::new(RealFileSystem),
            commands: Box::new(RealCommandRunner),
        }
    }
}

#[cfg(test)]
mod tests;
