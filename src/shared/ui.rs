//! *How* to render a message — the DI seam. Nothing outside
//! `events::PrintAction` should call this directly; commands go through
//! `EventBus`/`Updates` instead (see `shared::events`).

pub trait Ui {
    fn step(&self, msg: &str);
    fn warn(&self, msg: &str);
    fn success(&self, msg: &str);
    fn error(&self, msg: &str);
}

pub struct ConsoleUi;

impl Ui for ConsoleUi {
    fn step(&self, msg: &str) {
        println!("→ {msg}");
    }

    fn warn(&self, msg: &str) {
        println!("⚠ {msg}");
    }

    fn success(&self, msg: &str) {
        println!("✓ {msg}");
    }

    fn error(&self, msg: &str) {
        eprintln!("✗ {msg}");
    }
}

// `messages` is `Rc<RefCell<..>>` so tests can hold a cloned handle after
// this is moved into a `Box<dyn Ui>` and still observe what was rendered.
// Only exercised by #[cfg(test)] code in command modules; `cargo build`
// alone can't see those call sites.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Default)]
pub struct RecordingUi {
    pub messages: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

impl Ui for RecordingUi {
    fn step(&self, msg: &str) {
        self.messages.borrow_mut().push(format!("step: {msg}"));
    }

    fn warn(&self, msg: &str) {
        self.messages.borrow_mut().push(format!("warn: {msg}"));
    }

    fn success(&self, msg: &str) {
        self.messages.borrow_mut().push(format!("success: {msg}"));
    }

    fn error(&self, msg: &str) {
        self.messages.borrow_mut().push(format!("error: {msg}"));
    }
}
