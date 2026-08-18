//! Lifecycle events a command's execution goes through, and the seam for
//! reacting to them. Commands never emit `Started`/`Finished`/`Error`
//! directly — those are emitted structurally by `EventBus::run`, which
//! wraps a command's whole body, so the pairing (exactly one `Finished`
//! *or* `Error`, never neither, never both) can't be gotten wrong by hand.

use anyhow::Result;

use super::ui::Ui;

// `command` isn't read by `PrintAction` (it only cares about `message`),
// but it's there for the next `Action` that needs to filter/tag by which
// command emitted an event — not dead in the sense of "never should have
// existed," just not consumed by the one `Action` registered so far.
#[allow(dead_code)]
pub enum Event {
    Started {
        command: &'static str,
    },
    Updated {
        command: &'static str,
        message: String,
    },
    Warned {
        command: &'static str,
        message: String,
    },
    Finished {
        command: &'static str,
        message: String,
    },
    Error {
        command: &'static str,
        message: String,
    },
}

/// Reacts to one `Event`. A command's own `listeners.rs` registers a list
/// of these on its `EventBus` — today that's one action (print it) for
/// every command, but nothing about this trait assumes that stays true, or
/// stays the same across commands.
pub trait Action {
    fn handle(&self, event: &Event);
}

pub struct EventBus<'a> {
    actions: Vec<Box<dyn Action + 'a>>,
}

impl<'a> EventBus<'a> {
    pub fn new(actions: Vec<Box<dyn Action + 'a>>) -> Self {
        Self { actions }
    }

    fn emit(&self, event: Event) {
        for action in &self.actions {
            action.handle(&event);
        }
    }

    /// Runs a command's body as a lifecycle: emits `Started` first, hands
    /// the body an `Updates` handle for `updated`/`warned` progress
    /// signals, then emits exactly one of `Finished` (the body's `Ok`
    /// value becomes the finished message) or `Error` (from the body's
    /// `Err`, rendered via `{err:#}`).
    pub fn run(
        &self,
        command: &'static str,
        body: impl FnOnce(&Updates) -> Result<String>,
    ) -> Result<()> {
        self.emit(Event::Started { command });
        let updates = Updates { bus: self, command };
        match body(&updates) {
            Ok(message) => {
                self.emit(Event::Finished { command, message });
                Ok(())
            }
            Err(err) => {
                self.emit(Event::Error {
                    command,
                    message: format!("{err:#}"),
                });
                Err(err)
            }
        }
    }
}

/// Handed to a command's body by `EventBus::run` — the only way to emit
/// progress from inside one.
pub struct Updates<'bus, 'a> {
    bus: &'bus EventBus<'a>,
    command: &'static str,
}

impl Updates<'_, '_> {
    pub fn updated(&self, message: impl Into<String>) {
        self.bus.emit(Event::Updated {
            command: self.command,
            message: message.into(),
        });
    }

    pub fn warned(&self, message: impl Into<String>) {
        self.bus.emit(Event::Warned {
            command: self.command,
            message: message.into(),
        });
    }
}

/// Forwards every `Event` to a `Ui`. Borrows rather than owns — a
/// command's `listeners.rs` builds this fresh each `run()` from
/// `&ctx.ui`, so no cloning/sharing story is needed for a trait object
/// that only needs to live as long as the one command invocation using it.
pub struct PrintAction<'a> {
    ui: &'a dyn Ui,
}

impl<'a> PrintAction<'a> {
    pub fn new(ui: &'a dyn Ui) -> Self {
        Self { ui }
    }
}

impl Action for PrintAction<'_> {
    fn handle(&self, event: &Event) {
        match event {
            // Nothing to say yet — the first `Updated` a moment later
            // already tells the user what's starting. Still fires for any
            // *other* future Action that wants to know a command began
            // (timing, logging, ...) even though this one ignores it.
            Event::Started { .. } => {}
            Event::Updated { message, .. } => self.ui.step(message),
            Event::Warned { message, .. } => self.ui.warn(message),
            Event::Finished { message, .. } => self.ui.success(message),
            Event::Error { message, .. } => self.ui.error(message),
        }
    }
}
