use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{Context, Result};

pub trait FileSystem {
    fn write_file(&self, path: &Path, contents: &str) -> Result<()>;
}

pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn write_file(&self, path: &Path, contents: &str) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }
        std::fs::write(path, contents)
            .with_context(|| format!("failed to write file {}", path.display()))
    }
}

// Only exercised by #[cfg(test)] code in this and other modules; `cargo build`
// alone can't see those call sites.
// `written` is `Rc<RefCell<..>>` (not plain `RefCell<..>`) so tests can hold
// a cloned handle after the `InMemoryFileSystem` itself is moved into a
// `Box<dyn FileSystem>`, and still observe writes made through the trait
// object.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Default)]
pub struct InMemoryFileSystem {
    pub written: Rc<RefCell<HashMap<PathBuf, String>>>,
}

impl FileSystem for InMemoryFileSystem {
    fn write_file(&self, path: &Path, contents: &str) -> Result<()> {
        self.written
            .borrow_mut()
            .insert(path.to_path_buf(), contents.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests;
