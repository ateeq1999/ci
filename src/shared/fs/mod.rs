use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{Context, Result};

pub trait FileSystem {
    fn write_file(&self, path: &Path, contents: &str) -> Result<()>;

    /// `Ok(None)` when the file doesn't exist; `Err` for any other read
    /// failure (permissions, not valid UTF-8, ...).
    fn try_read_to_string(&self, path: &Path) -> Result<Option<String>>;
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

    fn try_read_to_string(&self, path: &Path) -> Result<Option<String>> {
        match std::fs::read_to_string(path) {
            Ok(contents) => Ok(Some(contents)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).with_context(|| format!("failed to read {}", path.display())),
        }
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

    fn try_read_to_string(&self, path: &Path) -> Result<Option<String>> {
        Ok(self.written.borrow().get(path).cloned())
    }
}

#[cfg(test)]
mod tests;
