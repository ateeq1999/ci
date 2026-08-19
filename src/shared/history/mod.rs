//! `ci/history.jsonl` — an append-only record, under the project's own
//! `ci/` folder (alongside `ci/config.json`), of every project-scoped
//! command `ci` has run against it and how it turned out. One JSON object
//! per line (not a single JSON array) so appending never requires
//! re-parsing/rewriting the whole file. Wired in as an `Action` — the same
//! seam `PrintAction` already uses — so every command gets history for
//! free just by including it in its `listeners::bus`, with no command
//! itself aware this exists.
//!
//! Deliberately scoped to commands that operate on a project directory
//! (`init`, `db`, `add`): `update` (which patches the `ci` binary itself,
//! not a project) has no project root to attach a history entry to, so it
//! doesn't build one.

use std::path::PathBuf;

use chrono::Utc;
use serde::Serialize;

use super::events::{Action, Event};
use super::fs::FileSystem;

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    Success,
    Error,
}

#[derive(Serialize)]
struct HistoryEntry<'a> {
    timestamp: String,
    command: &'a str,
    status: Status,
    message: &'a str,
}

/// Appends one line to `<root>/ci/history.jsonl` for every `Finished`/
/// `Error` event — the terminal outcomes a "history of commands and their
/// status" is actually about, not every intermediate `Updated`/`Warned`
/// progress message.
pub struct HistoryAction<'a> {
    fs: &'a dyn FileSystem,
    root: PathBuf,
}

impl<'a> HistoryAction<'a> {
    pub fn new(fs: &'a dyn FileSystem, root: impl Into<PathBuf>) -> Self {
        Self {
            fs,
            root: root.into(),
        }
    }

    fn append(&self, command: &str, status: Status, message: &str) {
        let entry = HistoryEntry {
            timestamp: Utc::now().to_rfc3339(),
            command,
            status,
            message,
        };
        // `unwrap_or_default` on the struct-to-string step can't actually
        // fail here (no floats/maps with non-string keys, nothing
        // serde_json rejects) — but a *write* failure (read-only fs, full
        // disk, ...) is real and must never turn an otherwise-successful
        // command into a failure just because its own history couldn't be
        // recorded. Best-effort, silently, on purpose.
        let Ok(line) = serde_json::to_string(&entry) else {
            return;
        };
        let path = self.history_path();
        let mut contents = self
            .fs
            .try_read_to_string(&path)
            .ok()
            .flatten()
            .unwrap_or_default();
        if !contents.is_empty() && !contents.ends_with('\n') {
            contents.push('\n');
        }
        contents.push_str(&line);
        contents.push('\n');
        let _ = self.fs.write_file(&path, &contents);
    }

    fn history_path(&self) -> PathBuf {
        self.root.join("ci/history.jsonl")
    }
}

impl Action for HistoryAction<'_> {
    fn handle(&self, event: &Event) {
        match event {
            Event::Finished { command, message } => {
                self.append(command, Status::Success, message)
            }
            Event::Error { command, message } => self.append(command, Status::Error, message),
            Event::Started { .. } | Event::Updated { .. } | Event::Warned { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests;
