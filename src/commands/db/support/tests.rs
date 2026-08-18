use super::*;
use crate::shared::events::{EventBus, PrintAction};
use crate::shared::ui::ConsoleUi;

// `guard_destructive` takes `&Updates`, which is only ever handed out
// inside `EventBus::run`'s closure — so testing it means going through
// `.run(...)` for real, the same path production uses.
fn with_bus<T>(f: impl FnOnce(&Updates) -> Result<T>) -> Result<T> {
    let ui = ConsoleUi;
    let bus = EventBus::new(vec![Box::new(PrintAction::new(&ui))]);
    let mut out = None;
    bus.run("test", |events| {
        out = Some(f(events)?);
        Ok(String::new())
    })?;
    Ok(out.expect("closure ran on Ok"))
}

#[test]
fn refuses_production_without_force() {
    let err =
        with_bus(|events| guard_destructive(events, "do a dangerous thing", true, false, true))
            .unwrap_err();
    assert!(err.to_string().contains("NODE_ENV=production"));
}

#[test]
fn allows_production_with_force() {
    with_bus(|events| guard_destructive(events, "do a dangerous thing", true, true, true)).unwrap();
}

#[test]
fn skips_prompt_when_yes_is_set() {
    // If this didn't skip the confirmation prompt, it would block on stdin.
    with_bus(|events| guard_destructive(events, "do a dangerous thing", true, false, false))
        .unwrap();
}

#[test]
fn warns_before_the_confirmation_prompt() {
    let ui = crate::shared::ui::RecordingUi::default();
    let messages = ui.messages.clone();
    let bus = EventBus::new(vec![Box::new(PrintAction::new(&ui))]);

    // No --yes: `confirm()` reads stdin, which under `cargo test` hits EOF
    // immediately rather than blocking, answering "no" and aborting — the
    // point of this test is only that the warning fired first.
    let _ = bus.run("test", |events| {
        guard_destructive(events, "drop everything", false, false, false)?;
        Ok(String::new())
    });

    let messages = messages.borrow();
    assert!(messages.iter().any(|m| m == "warn: This will drop everything."));
    assert!(messages.iter().any(|m| m.starts_with("error:")));
}
