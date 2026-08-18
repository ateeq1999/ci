# Plan: event-driven command output — `Ui` + lifecycle `EventBus`, both DI'd

Supersedes the previous versions of this file. Three decisions now locked
in:

1. Commands emit events; each command owns its own `listeners.rs`
   deciding which actions react to them.
2. Events are **lifecycle** events — `Started`/`Updated`/`Warned`/
   `Finished`/`Error` — not ad-hoc print calls. `Started` and
   `Finished`/`Error` are emitted automatically by wrapping a command's
   execution, not typed out by hand at the top and bottom of every `run()`.
3. Paths below live under `src/shared/`, matching the reorg already on
   disk (`context.rs`/`db_orm.rs`/`fs.rs`/`json_payload.rs`/`ui.rs` moved
   there from `src/` directly — `src/shared/events.rs` joins them).

Printing is the only action wired up anywhere today, but the mechanism —
and the fact that it's per-command, not one shared global registration —
doesn't change when a second action shows up later.

---

## Shape

```
src/shared/ui.rs        Ui (trait)      — *how* to render a message
                                           (console today). DI'd on Context.
src/shared/events.rs      Event           — Started/Updated/Warned/Finished/
                                             Error, each tagged with which
                                             command emitted it
                          Action (trait)  — reacts to one Event. The seam
                                             "actions to be executed" lives on.
                          EventBus        — fans an emitted Event out to
                                             whichever Actions it was built
                                             with; `run()` wraps a command's
                                             whole execution, auto-emitting
                                             Started/Finished/Error around it
                          PrintAction     — the one Action every command
                                             registers today: forwards each
                                             Event to `Ui`

src/commands/init/listeners.rs    — init's own registration: which
src/commands/update/listeners.rs    Actions react to *this command's*
src/commands/db/listeners.rs        events. Today: just PrintAction, for
                                     every command. The home for a future
                                     command-specific action without
                                     touching the other two commands or the
                                     shared bus/action infrastructure.
```

Commands depend on `ctx.ui` (DI'd, swappable) and their own
`listeners::bus(ctx)` (built from it). They never call `Ui` directly —
that's wrapped inside whatever `Action`s their own `listeners.rs`
registers, and they never emit `Started`/`Finished`/`Error` directly
either — that's the job of `EventBus::run`, which wraps their body.

---

## `Event` and `Action` — shared infrastructure, `src/shared/events.rs`

```rust
pub enum Event {
    Started { command: &'static str },
    Updated { command: &'static str, message: String },
    Warned { command: &'static str, message: String },
    Finished { command: &'static str, message: String },
    Error { command: &'static str, message: String },
}

/// Reacts to one `Event`. What a command's `listeners.rs` registers a list
/// of — today that's one action (print it) for every command, but nothing
/// about this trait assumes that stays true, or stays the same across
/// commands.
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
    /// signals, then emits `Finished` (the body's `Ok` value becomes the
    /// finished message — every command here already only returns `()` on
    /// success, so "the thing to say when done" is a natural stand-in for
    /// it) or `Error` (from the body's `Err`, via `{err:#}`) — never both.
    pub fn run(
        &self,
        command: &'static str,
        body: impl FnOnce(&Updates) -> anyhow::Result<String>,
    ) -> anyhow::Result<()> {
        self.emit(Event::Started { command });
        let updates = Updates { bus: self, command };
        match body(&updates) {
            Ok(message) => {
                self.emit(Event::Finished { command, message });
                Ok(())
            }
            Err(err) => {
                self.emit(Event::Error { command, message: format!("{err:#}") });
                Err(err)
            }
        }
    }
}

/// Handed to a command's body by `EventBus::run` — the only way to emit
/// progress from inside one, so `Started`/`Finished`/`Error` stay exactly
/// paired with the body that ran, never hand-typed and never forgotten.
pub struct Updates<'bus, 'a> {
    bus: &'bus EventBus<'a>,
    command: &'static str,
}

impl Updates<'_, '_> {
    pub fn updated(&self, message: impl Into<String>) {
        self.bus.emit(Event::Updated { command: self.command, message: message.into() });
    }
    pub fn warned(&self, message: impl Into<String>) {
        self.bus.emit(Event::Warned { command: self.command, message: message.into() });
    }
}

/// Forwards every Event to a `Ui`. Borrows rather than owns — a command's
/// `listeners.rs` builds this fresh each `run()` from `&ctx.ui`, so no
/// cloning/sharing story is needed for a trait object that only needs to
/// live as long as the one command invocation using it.
pub struct PrintAction<'a> {
    ui: &'a dyn crate::shared::ui::Ui,
}

impl<'a> PrintAction<'a> {
    pub fn new(ui: &'a dyn crate::shared::ui::Ui) -> Self {
        Self { ui }
    }
}

impl Action for PrintAction<'_> {
    fn handle(&self, event: &Event) {
        match event {
            // Nothing to say yet at this point — the first `Updated` a
            // moment later already tells the user what's starting. Still
            // fires for any *other* future Action that wants to know a
            // command began (timing, logging, ...) even though this one
            // ignores it.
            Event::Started { .. } => {}
            Event::Updated { message, .. } => self.ui.step(message),
            Event::Warned { message, .. } => self.ui.warn(message),
            Event::Finished { message, .. } => self.ui.success(message),
            Event::Error { message, .. } => self.ui.error(message),
        }
    }
}
```

`EventBus` is a concrete struct, not a trait — the DI/swappability lives
in *which `Action`s it's built with* (decided per-command, in each
`listeners.rs`), not in swapping the dispatch loop itself. If some future
need shows up for swapping dispatch order/filtering rather than just
which actions run, promote it to a trait then; nothing above forecloses
it.

---

## `Ui` — the rendering seam, `src/shared/ui.rs`

```rust
pub trait Ui {
    fn step(&self, msg: &str);
    fn warn(&self, msg: &str);
    fn success(&self, msg: &str);
    fn error(&self, msg: &str);
}

pub struct ConsoleUi;

impl Ui for ConsoleUi {
    fn step(&self, msg: &str) { println!("→ {msg}"); }
    fn warn(&self, msg: &str) { println!("⚠ {msg}"); }
    fn success(&self, msg: &str) { println!("✓ {msg}"); }
    fn error(&self, msg: &str) { eprintln!("✗ {msg}"); }
}
```

Test double, for swapping into `Context` the same way
`InMemoryFileSystem`/`NoopCommandRunner` already get swapped in:

```rust
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Default)]
pub struct RecordingUi {
    pub messages: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

impl Ui for RecordingUi {
    fn step(&self, msg: &str) { self.messages.borrow_mut().push(format!("step: {msg}")); }
    fn warn(&self, msg: &str) { self.messages.borrow_mut().push(format!("warn: {msg}")); }
    fn success(&self, msg: &str) { self.messages.borrow_mut().push(format!("success: {msg}")); }
    fn error(&self, msg: &str) { self.messages.borrow_mut().push(format!("error: {msg}")); }
}
```

`Ui` is where the DI/test-swap point actually lives — since every
command's `listeners.rs` registers the same `PrintAction` wrapping
whatever `ctx.ui` is, swapping `ctx.ui` to `RecordingUi` is enough for
tests to assert on output without needing a separate test `Action` too.

---

## `Context` changes (`src/shared/context.rs`)

```rust
pub struct Context {
    pub fs: Box<dyn FileSystem>,
    pub commands: Box<dyn CommandRunner>,
    pub ui: Box<dyn crate::shared::ui::Ui>,
}

impl Context {
    pub fn real() -> Self {
        Self {
            fs: Box::new(RealFileSystem),
            commands: Box::new(RealCommandRunner),
            ui: Box::new(crate::shared::ui::ConsoleUi),
        }
    }
}
```

## Each command's `listeners.rs`

```rust
// src/commands/init/listeners.rs
use crate::shared::context::Context;
use crate::shared::events::{EventBus, PrintAction};

pub fn bus(ctx: &Context) -> EventBus<'_> {
    EventBus::new(vec![Box::new(PrintAction::new(ctx.ui.as_ref()))])
}
```

`src/commands/update/listeners.rs` and `src/commands/db/listeners.rs` are
identical today — that repetition is deliberate, not an oversight to
dedupe: each is the file that command's future second action gets added
to, without the other two commands' behavior changing.

## Call sites

```rust
// src/commands/init/mod.rs
mod listeners;

pub fn run(args: &Args, ctx: &Context) -> Result<()> {
    listeners::bus(ctx).run("init", |events| {
        let name = args.name.as_deref().context(...)?;
        let root = PathBuf::from(name);

        events.updated(&format!("Scaffolding a NestJS project in {} ...", root.display()));
        for (path, contents) in templates::starter_files(name, args.orm, args.driver)? {
            ctx.fs.write_file(&root.join(path), &contents)?;
        }
        // ... ci/config.json write ...

        if !args.skip_git {
            events.updated("Running git init");
            ctx.commands.run("git", &["init"], &root)?;
        }
        if !args.skip_install {
            events.updated(&format!("Installing dependencies with {}", args.package_manager.command()));
            ctx.commands.run(args.package_manager.command(), &["install"], &root)?;
        }

        Ok(format!("Created NestJS project in {}", root.display()))
    })
}
```

No more manually paired "print success at the end / hope every early
`return Err(...)` still gets reported right." `?` inside the closure falls
through to `EventBus::run`'s `Err` arm automatically, which is the
`Error` event firing — the lifecycle pairing is structural, not something
each command has to remember to do correctly.

`db::mod.rs` is the same reshape: each of `init`/`migrate`/`migrate
fresh`/`migrate refresh`/`migrate rollback` becomes one
`listeners::bus(ctx).run("db", |events| { ... })` call (or a per-action
command tag — `"db migrate fresh"` etc — see "Open question" below).
`guard_destructive`'s warning before the confirmation prompt becomes
`events.warned(...)`, called from inside the closure before the
destructive work.

**`update::run`** currently takes only `args: &Args`, no `ctx` — it needs
one now, plus its own `mod listeners;` and the same
`listeners::bus(ctx).run("update", |events| { ... })` wrapping. Update
its single call site in `commands::run`.

**`main.rs`**: today it calls `ui::error(...)` directly on failure — that
whole branch goes away. `commands::run`'s own dispatch already runs
inside each command's `EventBus::run`, so by the time an `Err` reaches
`main`, the `Error` event already fired and `PrintAction` already printed
it. `main` just needs to exit nonzero, no second print:

```rust
fn main() {
    if args::wants_help_all() { ... }
    let cli = Cli::parse();
    let ctx = Context::real();
    if commands::run(&cli, &ctx).is_err() {
        std::process::exit(1);
    }
}
```

---

## Testing

Tests swap `ctx.ui` for `RecordingUi`, same shape as swapping in
`InMemoryFileSystem`/`NoopCommandRunner` today:

```rust
let ui = RecordingUi::default();
let messages = ui.messages.clone();
let ctx = Context {
    fs: Box::new(InMemoryFileSystem::default()),
    commands: Box::new(NoopCommandRunner::default()),
    ui: Box::new(ui),
};

run(&args, &ctx).unwrap();

assert!(messages.borrow().iter().any(|m| m.starts_with("success:")));
```

Because `Started`/`Finished`/`Error` are structural now (emitted by
`EventBus::run`, not typed by hand), a test can assert the pairing itself
holds — e.g. a failing case always produces an `error:` message and never
a `success:` one — without that being something each command's author
had to get right by hand.

---

## Migration steps

1. `src/shared/events.rs` — `Event`, `Action`, `EventBus` (with `run`),
   `Updates`, `PrintAction`.
2. `src/shared/ui.rs` — trim to the `Ui` trait + `ConsoleUi` + `RecordingUi`
   (drop the four free functions; nothing outside `PrintAction` should
   call `Ui` directly anymore).
3. `Context` gains `ui: Box<dyn Ui>`; `Context::real()` wires `ConsoleUi`.
4. One `listeners.rs` per command — `init`, `update`, `db` — each
   exporting `pub fn bus(ctx: &Context) -> EventBus<'_>` registering
   `PrintAction` (identical across all three today, on purpose).
5. Give `update::run` a `&Context` parameter; update its call site in
   `commands::run`.
6. Reshape each command's `run()` body into the `listeners::bus(ctx)
   .run("<name>", |events| { ... })` closure form, replacing every
   `ui::step/warn/success/error(...)` call with
   `events.updated/warned(...)` (the closure's `Ok(String)` /
   propagated `Err` replace the old manual `success`/`error` calls
   entirely).
7. `main.rs`: build `Context::real()`, pass `&ctx` into `commands::run`,
   delete the direct `ui::error(...)` call — `Error` already printed by
   the time `main` sees the `Err`.
8. Update every existing test's `Context { fs, commands }` literal to
   also set `ui: Box::new(RecordingUi::default())` (or `ConsoleUi` where
   a test doesn't care about output) — mechanical, same shape as when
   `driver: DrizzleDriver::Pg` got added to every `init::Args` literal.
9. Optionally, a few new tests asserting on `RecordingUi`'s captured
   messages per command (not required for the migration to be complete,
   but the reason this was worth doing over leaving `ui::*` as free
   functions).

---

## Open question worth deciding before step 6

`db`'s five operations (`init`, `migrate`, `migrate fresh`, `migrate
refresh`, `migrate rollback`) currently share one `run()` dispatching on
`args.command`. Wrapping the *whole* `db::run` in one
`listeners::bus(ctx).run("db", ...)` call means `Started`/`Finished` fire
once per invocation regardless of which of the five ran — fine, since
exactly one always runs per invocation anyway. The only decision is the
`command` tag string: plain `"db"` for every operation, or something more
specific per action (`"db migrate fresh"`, `"db migrate rollback"`, ...)
so a future `Action` filtering on `event` can tell them apart without
inspecting the message text. Leaning toward the specific-tag form — it's
a few more `&'static str` literals in `db::mod.rs`'s match arms, costs
nothing today, and is exactly the kind of thing that's annoying to
retrofit once some future action depends on the coarser tag.

---

## Explicitly not building yet

- **A second `Action` on any command.** Every `listeners.rs` registers
  exactly `PrintAction` today. The structure (one file per command) exists
  so adding a command-specific one later is additive — it doesn't mean
  one is needed now.
- **Multiple simultaneous `Ui` implementations at once** (e.g. print *and*
  log to a file). `EventBus`/`Action` already support registering more
  than one action; not exercised until something needs it.
- **Nested/child lifecycles** (e.g. `db init` internally running two
  sub-steps that each want their own Started/Finished pair, not just
  `Updated` messages). Everything today is one flat `Started` →
  `Updated`/`Warned`* → `Finished`/`Error` per command invocation; nesting
  would need `EventBus::run` calls to compose, which they don't do yet.
