# Plan: `ci add` — wire NestJS techniques into an existing project

## Status: `validation`/`cache`/`schedule`/`queue`/`logger` all shipped in v0.1.2, plus cross-cutting `ci/history.jsonl` command history

## Cross-cutting: `ci/history.jsonl` — a record of every command run against a project

Not an `add` subcommand — a small addition to the event system every
command (`init`, `db`, `add`) already runs through. Under the project's
own `ci/` folder, alongside `ci/config.json`, `ci` now keeps an
append-only log of every project-scoped command it has run and how it
turned out: one JSON object per line (`ci/history.jsonl`, not a single
JSON array), so appending never means re-parsing/rewriting the whole
file. Each line: `{"timestamp": "...", "command": "add cache", "status":
"success"|"error", "message": "..."}` — `command`/`message` are exactly
the strings the event system already threads through `EventBus::run` and
`Finished`/`Error` events, so nothing new had to be invented to know what
to log.

**Why an `Action`, not a bolt-on wrapper.** `src/shared/events.rs` already
has the exact seam for this — `Action`, the trait `PrintAction` (prints
progress) implements, with a comment on `Event` foreshadowing it almost
verbatim: "not dead... just not consumed by the one `Action` registered
so far... (timing, logging, ...)." `HistoryAction` (`src/shared/history/
mod.rs`) is a second `Action`, registered alongside `PrintAction` in every
project-scoped command's `listeners::bus`. Every command gets history for
free just by building its bus the way it already did; no command's own
logic changed to support this.

**Only `Finished`/`Error`, not every event.** "History of commands and
their status" is about outcomes, not a full progress transcript —
`Started`/`Updated`/`Warned` are ignored. One entry per command
invocation, appended when it's over.

**Scoped to commands with a project.** `update` (self-updates the `ci`
binary itself via `self_update`, never touches a project directory) has
no root to attach a history entry to, so its `listeners.rs` was left
alone — no `HistoryAction`, no history. `init`, `db`, and `add` all
operate on a project root and got one.

**`init`'s root wasn't known before the event lifecycle started — until
now.** Every other command computes `root` before building its bus
(`add`/`db`: `std::env::current_dir()`). `init` didn't: `root` came from
`args.name`, validated *inside* `bus.run("init", |events| { ... })`'s
closure, via `args.name.as_deref().context(...)?`. That meant `Started`
already fired before `root` existed, and if `name` was missing, `root`
never would. Fixed by hoisting that validation (and the `PathBuf::from
(name)` computation) *above* `listeners::bus(ctx, &root).run(...)` —
`listeners::bus` now always has a root to build `HistoryAction` with. If
`name` truly is missing, there's no project to attach a history entry to
at all — that failure is still reported exactly as before (`` `name` is
required... ``), just without ever constructing a bus for it. No test
depended on the old ordering (`init::tests::errors_when_name_missing`
only asserts on the error text), so this was a safe, small refactor.

**Best-effort, deliberately.** A history *write* failure (read-only
filesystem, full disk, permissions) must never turn an otherwise
successful command into a failure just because its own record of success
couldn't be written. `HistoryAction::append` swallows both the
(practically-never-fails) serialization step and the real write step
silently, on purpose — this is the one place in the whole codebase that
intentionally ignores a `Result` rather than propagating it, and it's
called out with a comment explaining why, not left as an unexplained
`let _ =`.

**Gitignored, not committed.** `ci/config.json` is team-shared config and
stays tracked; `ci/history.jsonl` changes on every single `ci` invocation
and is fundamentally a local execution log, not shared state — same
category `*.log` is already in. `templates/init/.gitignore` gained one
new line, `ci/history.jsonl`.

Verified: 5 new tests in `src/shared/history/tests.rs` (a `Finished`
event records a `"success"` entry, an `Error` event records `"error"`,
`Started`/`Updated`/`Warned` are all ignored, multiple commands append
rather than each overwriting the file, and each line parses as its own
independent JSON object — this is JSONL, not a JSON array). Plus a real
run: `ci init` → `ci add validation` → `ci add logger` → (`.env` moved
aside) `ci db migrate fresh` against an actual scaffolded project,
confirming `ci/history.jsonl` accumulated one correctly-shaped line per
command — three `"success"` entries and one `"error"` entry with the
real `.env not found` message — in call order.

---

## `ci add logger`: shipped in v0.1.2

Per [the docs](https://docs.nestjs.com/techniques/logger). **Default
logger is console** (per the ask) — NestJS's own built-in `ConsoleLogger`,
not Winston/Pino/any external logging library.

**Dependencies: none.** `ConsoleLogger`/`Injectable`/`Global`/`Module` all
ship in `@nestjs/common`, already present in every `init`-generated
project. First `add` subcommand that never shells out to a package
manager at all.

**A standalone module, matching `db`'s own shape** (the ask's explicit
comparison) — `src/database/database.module.ts` is a small `@Global()
@Module({ providers: [...], exports: [...] })` wrapping the DB connection
behind one importable module; `ci add logger` writes the same shape for
logging:

```typescript
// src/logger/logger.service.ts
import { ConsoleLogger, Injectable } from '@nestjs/common';

@Injectable()
export class AppLogger extends ConsoleLogger {}
```

```typescript
// src/logger/logger.module.ts
import { Global, Module } from '@nestjs/common';
import { AppLogger } from './logger.service';

@Global()
@Module({
  providers: [AppLogger],
  exports: [AppLogger],
})
export class LoggerModule {}
```

`AppLogger extends ConsoleLogger` rather than just injecting the built-in
`Logger` directly (which the docs show needs no module at all) — the
whole point of "standalone module" is a single place later customization
(timestamps, JSON formatting, log-level filtering) can live without
touching every file that logs, matching `DatabaseModule`'s role as the
one place DB config lives.

**`app.module.ts` patch** — import + array entry, idempotent on
`"LoggerModule,"` for the array insertion (not the looser `"LoggerModule"`
— that substring is also in the import line, which would make the array
check pass falsely and skip its own insertion; same trap `cache`/
`schedule` already avoid with `"CacheModule.registerAsync"`/
`"ScheduleModule.forRoot"`):

```typescript
import { LoggerModule } from './logger/logger.module';
```

```typescript
LoggerModule,
```

**`main.ts` patch — two insertions, at two structurally distinct anchors,
not the same one `validation` uses.** `app.useLogger(app.get(AppLogger))`
needs to run somewhere between `NestFactory.create` and `app.listen`, but
anchoring it on `NestFactory.create` (like `validation`'s pipe already
does) would hit the exact same "two subcommands, one fixed anchor" bug
`insert_after_last_import` was built to fix for `app.module.ts`'s
imports — whichever of `validation`/`logger` ran second would land its
statement first, ahead of the other, regardless of actual run order.
Rather than inventing another dynamic "last statement" primitive,
`logger`'s statement anchors on `await app.listen(` instead — a distinct,
stable anchor that never moves — and a new `insert_before` primitive
inserts right *above* it:

```rust
// src/commands/add/patch.rs
/// Inserts `line` right *before* the first line containing `anchor`.
/// Complements `insert_after`/`insert_after_last_import` for the case
/// where the natural, order-stable anchor is something that must stay
/// *last* (e.g. `main.ts`'s `await app.listen(...)`) rather than
/// something that must stay first.
pub fn insert_before(
    ctx: &Context,
    path: &Path,
    anchor: &str,
    already_present_marker: &str,
    line: &str,
) -> Result<bool> { /* mirrors insert_after, inserting above the anchor line instead of below it */ }
```

Because `validation`'s pipe anchors high (right after `NestFactory.create`)
and `logger`'s `useLogger` anchors low (right before `app.listen`), the
two can never collide or reorder each other no matter which runs first —
`main.ts` ends up with `NestFactory.create` → pipe → `useLogger` →
`listen` every time. Verified directly: a real run produced exactly that
order.

`import { AppLogger } from './logger/logger.service';` uses
`insert_after_last_import` (retrofitting `main.ts`'s import handling to
the same primitive `app.module.ts` uses — see the cross-cutting fix
below) rather than the fixed `NestFactory` anchor `validation`'s own
import insertion used to use.

**No `bufferLogs: true` on `NestFactory.create`, deliberately not built.**
The docs' full pattern for capturing Nest's own internal bootstrap
messages through a custom logger needs `NestFactory.create(AppModule, {
bufferLogs: true })` plus an early `app.useLogger(...)` call. The first
half means *modifying* an existing line's arguments, not inserting new
content next to it — every patch primitive `ci` has (`insert_after`/
`insert_before`/`insert_after_last_import`/`insert_into_array`/
`append_line`) only ever inserts, none rewrites a line in place. Skipped
for v1: the gap is genuinely small (Nest's own internal startup messages
already print via the default console logger either way; only messages
Nest itself emits *before* `app.useLogger()` runs would differ, and
`AppLogger` doesn't change formatting from stock `ConsoleLogger` yet), and
"edit an existing line" is new, different-shaped capability worth its own
justification later rather than smuggling in alongside this subcommand.

**No example log call, no `LOG_LEVEL` env var, no per-context
configuration.** Same boundary every other `add` subcommand already
draws: `ci add logger`'s job ends at making `AppLogger` active and
injectable; what a project logs, at what level, is business-specific.

**Writing genuinely new files needs its own idempotency shape.**
`logger` is the first `add` subcommand that creates brand-new files
(`logger.service.ts`/`logger.module.ts`) rather than only patching
existing ones — `insert_after`-style "does this exact content already
exist" doesn't apply to a whole file. New primitive:

```rust
// src/commands/add/patch.rs
/// Writes `contents` to `path` only if nothing is there yet. Returns
/// `Ok(false)` without touching the file if something's already there,
/// so re-running doesn't clobber whatever a project did with the file
/// since it was first generated.
pub fn write_file_if_absent(ctx: &Context, path: &Path, contents: &str) -> Result<bool> { /* ... */ }
```

Verified directly: hand-edit `logger.service.ts` after the first
`ci add logger` run, run it again, confirm the hand edit survives
untouched.

Naming: **`logger`, not `log`.** `log` reads as a verb/action (matching
neither the docs' title, "Logger," nor this family's established pattern
of naming the *artifact* being wired in — `validation`, `cache`,
`schedule`, `queue` are all nouns for the thing that gets active, never a
verb). `logger` also matches the class this subcommand actually creates,
`LoggerModule`, the same way `cache` matches `CacheModule`.

Verified: `src/commands/add/logger/tests.rs` (writes both files and
patches both `app.module.ts`/`main.ts` correctly against real `init`
output; a stacking test running `validation` then `logger` and asserting
`NestFactory.create` → pipe → `useLogger` → `listen` order; an
idempotency test; the hand-edit-survives test above), plus a real
`ci init` → `ci add validation` → `ci add logger` run confirming zero
`npm install` calls, correct file contents, and the exact statement
ordering above in the generated `main.ts`.

### Cross-cutting fix that came out of building `logger`: `main.ts` imports needed the same treatment as `app.module.ts`'s

`validation`'s own import insertion (`import { ValidationPipe } from
'@nestjs/common';`) used to anchor on the literal `"import { NestFactory }
from '@nestjs/core';"` line — the same fixed-anchor shape `cache` used to
use for `app.module.ts`, and the same class of bug: once `logger` also
needed to add an import to `main.ts`, two subcommands anchoring on the
same fixed line would collide exactly like `app.module.ts`'s imports
used to. Rather than reinvent the fix, `validation`'s import insertion
was retrofitted onto `insert_after_last_import` (already generic — it
works on any file, not just `app.module.ts`) at the same time `logger`
was built, so it's now *the* way any `add` subcommand adds an import
line anywhere, full stop.

---

## `schedule`/`queue`: shipped in v0.1.2

Built as designed below. `insert_after_last_import` added to `patch.rs`
first (the ordering-bug fix), with `cache`'s import insertion retrofitted
onto it in the same change; `schedule` and `queue` then built on top,
each its own self-contained folder (`mod.rs`+`tests.rs`), matching
`validation`/`cache`. `Command::{Schedule, Queue}` wired into
`add::args`/`add::mod`.

Verified: plus a real end-to-end run: `ci init` → `ci add cache` → `ci add
schedule` → `ci add queue` against the actual npm registry, confirming
real resolved versions (`@nestjs/schedule ^6.1.3`, `@nestjs/bullmq
^11.0.5`, `bullmq ^6.1.2`, `ioredis ^6.0.0`), a single `REDIS_URL=` line
in `.env`/`.env.example` shared correctly between `cache` and `queue`,
imports landing in run order (`DatabaseModule` → `CacheModule` →
`ScheduleModule` → `BullModule`) rather than colliding on one fixed
anchor, and re-running all three a second time reporting "already
configured" with zero duplicate imports/module entries/env lines.

## `validation`/`cache`: shipped in v0.1.2

Built as designed below, then adjusted post-ship per follow-up feedback —
**`ci add caching` renamed to `ci add cache`**, **the example DTO
dropped** from `ci add validation` (nothing wires it into a route, so it
was inert), and **dependency installation replaced**: `patch.rs`'s
original `add_dependencies` (parse `package.json`, merge in a
hand-guessed semver range) is gone, replaced by
`install_dependencies` + `detect_package_manager` — shells out to the
project's real package manager (`npm install <pkgs>`, or `pnpm add`/`yarn
add` — npm alone doesn't use the `add` verb) so it resolves genuine
current versions instead of this tool guessing them. Which package
manager to use is read from `ci/config.json`'s `packageManager` field
(the same file `db::detect` already reads `orm`/`driver` from), falling
back to npm if the file's missing or unparseable.
`PackageManager` moved from `commands::init::args` to
`shared::package_manager` so both `init` (writing it) and `add` (reading
it) share one definition.

`src/commands/add/patch.rs`, `validation/` and `cache/` each
self-contained (own `mod.rs` + `tests.rs`, matching `db`'s per-subcommand
folders), `Commands::Add` wired into `args`/`commands::mod`. Redis is `ci
add cache`'s default store as approved — `CacheModule.registerAsync`
reading `process.env.REDIS_URL`, `@keyv/redis` installed alongside
`@nestjs/cache-manager`/`cache-manager`, `REDIS_URL=redis://localhost:6379`
appended to `.env`/`.env.example`.

Supersedes the previous version of this file (the event-driven `Ui`/
`EventBus` plan — shipped, see `src/shared/events.rs`/`src/shared/ui.rs`
and every command's `listeners.rs`; this doc's job there is done — and
now the seam that `ci/history.jsonl` above hangs off).

## Goal

A new top-level command, `ci add <technique>`, for wiring a NestJS
technique into a project `ci init` already created:

```
ci add validation   # https://docs.nestjs.com/techniques/validation      — shipped
ci add cache        # https://docs.nestjs.com/techniques/caching         — shipped
ci add schedule     # https://docs.nestjs.com/techniques/task-scheduling — shipped
ci add queue        # https://docs.nestjs.com/techniques/queues          — shipped
ci add logger       # https://docs.nestjs.com/techniques/logger          — shipped
```

`add` is deliberately named for growth — same role `db` plays for
database operations, but for "wire technique X into this project."
Future candidates (not built now): `ci add swagger`, `ci add throttling`,
`ci add health-check`. Each becomes its own self-contained folder under
`src/commands/add/`, the same shape `db`'s five subcommands already use.

### Naming: `schedule`/`queue`/`logger`, not `task`/`crons`/`queues`/`log`

The ask floated `ci add task`, `ci add crons`, `ci add queues` for the
first pair, and `ci add logger`/`ci add log` for the third. Went with the
following, matching this project's own precedent (`caching` → `cache`
after the first round of feedback — singular, short, matches the npm
package name, and always names the *artifact*, never an action):

- **`schedule`, not `task` or `crons`.** NestJS's own doc title and npm
  package are both "Task Scheduling" / `@nestjs/schedule` — `schedule`
  matches the package name exactly. `crons` undersells the feature (the
  module also covers `@Interval()` and `@Timeout()`, not just cron
  expressions) and is grammatically off as a subcommand noun. `task` is
  the vaguest of the three — could mean anything in a CLI.
- **`queue`, not `queues`.** Singular, matching `cache` (not `caches`)
  and `db`/`add` themselves. The underlying feature is plural-natured
  (multiple named queues), but the *subcommand* wires up one thing: queue
  infrastructure for the project.
- **`logger`, not `log`.** `log` reads as a verb/action, breaking the
  pattern every other subcommand follows (name the artifact: the
  `ValidationPipe`, the `CacheModule`, the `ScheduleModule`, the
  `BullModule`). `logger` matches both the docs' title ("Logger") and the
  class this subcommand actually creates, `LoggerModule`.

---

## The real problem: `ci` has never edited an existing file

Every command so far only ever *writes* files — `init` renders fresh
templates, `db`'s destructive operations shell out to other tools. Not
one line of this codebase modifies a file that's already there.
`ci add validation` needs to edit `src/main.ts` (insert a
`ValidationPipe` global pipe); `ci add cache` needs to edit
`src/app.module.ts` (insert `CacheModule` into `imports`). Both need
`package.json` updated with new dependencies. This is new capability, not
a mechanical reuse of what `init`/`db` already do — worth its own section
before the subcommands, since all of them depend on it.

### Dependencies: shell out to the real package manager, don't guess versions

Original draft of this section proposed parsing `package.json` as JSON
and merging in hand-picked semver ranges directly. Shipped differently,
per follow-up feedback: shell out to whichever package manager the
project is configured for (`npm install <pkgs>` — npm has no separate
`add` verb — or `pnpm add`/`yarn add`, which reserve `install` for
"install from the lockfile only"), and let *it* resolve and write real
current versions into `package.json`. Confirmed better in practice: real
runs resolved `class-validator ^0.15.1`, `@nestjs/cache-manager ^3.1.3`,
`@nestjs/schedule ^6.1.3`, `@nestjs/bullmq ^11.0.5` — all newer than this
plan's original guesses. `ci add logger` is the one exception: it needs
no dependencies at all, so it never calls `install_dependencies`.

```rust
// src/commands/add/patch.rs
pub fn detect_package_manager(ctx: &Context, root: &Path) -> PackageManager {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ProjectConfig {
        #[serde(default)]
        package_manager: PackageManager,
    }
    ctx.fs
        .try_read_to_string(&root.join("ci/config.json"))
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<ProjectConfig>(&raw).ok())
        .map(|config| config.package_manager)
        .unwrap_or_default() // npm, if ci/config.json is missing or unparseable
}

pub fn install_dependencies(
    ctx: &Context,
    root: &Path,
    package_manager: PackageManager,
    packages: &[&str],
) -> Result<()> {
    let mut args = vec![package_manager.add_verb()];
    args.extend(packages);
    ctx.commands.run(package_manager.command(), &args, root)
}
```

`PackageManager` (with its new `add_verb()`) moved from
`commands::init::args` to `shared::package_manager` so both `init`
(writing `ci/config.json`'s `packageManager` field) and `add` (reading it
back) share one definition.

### `main.ts`/`app.module.ts`: anchor-text insertion, not a TS parser

No TS AST tooling in Rust worth pulling in for a handful of insertion
points. But `ci` already knows *exactly* what these files look like — it
wrote them with `init`'s templates. Anchor on a known line from that
template, not a general parse:

```rust
// src/commands/add/patch.rs
/// Inserts `line` right after the first line containing `anchor`, unless
/// `already_present_marker` is already in the file — idempotent, and
/// errors clearly instead of guessing if the anchor isn't found (e.g.
/// someone hand-edited the file away from what `init` generated).
pub fn insert_after(
    ctx: &Context,
    path: &Path,
    anchor: &str,
    already_present_marker: &str,
    line: &str,
) -> Result<bool> { /* ... */ }
```

Returns `Ok(false)` for "already there" rather than erroring — `ci add
validation` run twice should say "already configured," not fail.

### Multiple subcommands patching the same file: anchor on the *last* import, not a fixed line

Found while designing `ci add queue`, before writing any code for it:
`cache` inserted its import line right after one fixed anchor —
`import { DatabaseModule } from './database/database.module';`. If
`schedule` and `queue` did the same, running more than one of
`cache`/`schedule`/`queue` against the same project would insert every
subcommand's import at that *same* fixed position — each new `add` run
would land its import right after `DatabaseModule`, ahead of whatever a
previous run had already put there, instead of appending after it. Still
syntactically valid TypeScript, but the insertion order silently depends
on nothing to do with the user's actual `ci add` sequence.

Fix: a primitive that anchors dynamically on whichever import line is
currently *last*, not on a fixed string:

```rust
// src/commands/add/patch.rs
/// Inserts `lines` right after the last line starting with `import `,
/// rather than a fixed anchor — so each successive `ci add` subcommand's
/// import lands after whatever the previous one inserted, not always in
/// the same fixed spot. Same idempotency-marker contract as `insert_after`.
pub fn insert_after_last_import(
    ctx: &Context,
    path: &Path,
    already_present_marker: &str,
    lines: &str,
) -> Result<bool> { /* rposition on l.trim_start().starts_with("import "), insert after it */ }
```

Retrofitted onto **both** files that see multiple subcommands:
`cache`/`schedule`/`queue`/`logger`'s `app.module.ts` import insertions,
and (once `logger` needed a second `main.ts` import) `validation`/
`logger`'s `main.ts` import insertions too — see `ci add logger`'s
section above for how that second retrofit came about.

`insert_into_array` (the `imports: [...]` array insertion) needs no
equivalent treatment — array insertion already targets "right after `[`,"
which is inherently order-stable across multiple subcommands: each new
entry lands at the front of the array, and array element order doesn't
carry the same "reads top-to-bottom as written" expectation import
statements do.

### Statements that must stay *last*, not first: `insert_before`

`app.module.ts`'s imports and `main.ts`'s original single insertion
(`validation`'s pipe) both wanted "stack after whatever came before,"
solved by anchoring dynamically on the last import. `ci add logger`'s
`app.useLogger(...)` call is different: it needs to run somewhere between
`NestFactory.create` and `app.listen`, and rather than build a second
dynamic "last statement" tracker, it anchors on `await app.listen(` — a
distinct, stable anchor no other subcommand touches — and a new
`insert_before` inserts directly above it:

```rust
// src/commands/add/patch.rs
pub fn insert_before(
    ctx: &Context,
    path: &Path,
    anchor: &str,
    already_present_marker: &str,
    line: &str,
) -> Result<bool> { /* mirrors insert_after, inserting above the anchor line instead of below it */ }
```

Because `validation`'s pipe anchors high and `logger`'s `useLogger`
anchors low, on two different lines that never move relative to each
other, the two can't collide regardless of which subcommand runs first —
no dynamic tracking needed for this case.

### Writing brand-new files: `write_file_if_absent`

`ci add logger` is the first `add` subcommand that creates whole new
files (`src/logger/logger.service.ts`/`logger.module.ts`) rather than
only patching existing ones. None of the insertion primitives apply to
"does this whole file already exist" — a dedicated primitive:

```rust
// src/commands/add/patch.rs
pub fn write_file_if_absent(ctx: &Context, path: &Path, contents: &str) -> Result<bool> {
    if ctx.fs.try_read_to_string(path)?.is_some() {
        return Ok(false);
    }
    ctx.fs.write_file(path, contents)?;
    Ok(true)
}
```

Returns `Ok(false)` (not an overwrite) when the file's already there —
re-running `ci add logger` must never clobber hand edits made to
`logger.service.ts` since it was first generated. Verified directly with
a test that hand-edits the file, re-runs, and asserts the hand edit
survives untouched.

---

## `ci add validation`

Per [the docs](https://docs.nestjs.com/techniques/validation):

**Dependencies:** `class-validator`, `class-transformer` (`ValidationPipe`
itself ships in `@nestjs/common`, already a dependency).

**`main.ts` patch** — insert right after
`const app = await NestFactory.create(AppModule);`:

```typescript
app.useGlobalPipes(
  new ValidationPipe({
    whitelist: true,
    forbidNonWhitelisted: true,
    transform: true,
  }),
);
```

Plus `import { ValidationPipe } from '@nestjs/common';`, via
`insert_after_last_import` (retrofitted from a fixed `NestFactory` anchor
once `logger` needed a second `main.ts` import — see above).

**No example DTO.** Originally planned as an unwired
`src/common/dto/example.dto.ts` "just showing the pattern," but dropped
before shipping: nothing wires it into a route (this tool has no
`generate controller` to attach it to non-arbitrarily), so it would just
be inert dead code sitting in every project this touches. `ci add
validation`'s whole job ends at making the pipe actually active.

Whitelist/transform options match the docs' "common production config" —
worth confirming as this project's default rather than the docs' bare
`new ValidationPipe()`, since a scaffolding tool should hand over
something production-shaped, not the minimal example.

---

## `ci add cache`

Per [the docs](https://docs.nestjs.com/techniques/caching).
**Redis is the default store** (approved) — not the bare in-memory
`CacheModule.register()` the docs lead with.

**Dependencies:** `@nestjs/cache-manager`, `cache-manager`, `@keyv/redis`.
No `keyv`/`KeyvCacheableMemory` needed — those are for the docs' *multi*-
store example (in-memory + Redis fallback layered together); a
Redis-only default has exactly one entry in `stores`, `new KeyvRedis(...)`
directly, no `Keyv` wrapper around it.

**`.env.example`/`.env` gets a new line**, same append-only shape as
`ci init` already writing `DATABASE_URL`:

```
REDIS_URL=redis://localhost:6379
```

**`app.module.ts` patch** — two insertions, both idempotent-checked on
`"CacheModule"`/`"CacheModule.registerAsync"`:
1. `import { CacheModule } from '@nestjs/cache-manager';` and
   `import { KeyvRedis } from '@keyv/redis';`, via `insert_after_last_import`.
2. Into the `imports: [...]` array:
   ```typescript
   CacheModule.registerAsync({
     isGlobal: true,
     useFactory: async () => ({
       stores: [new KeyvRedis(process.env.REDIS_URL ?? 'redis://localhost:6379')],
     }),
   }),
   ```
   `registerAsync`+`useFactory`, not the synchronous `register()` the
   docs lead with — needed either way once the store depends on an env
   var read at startup, redis-default or not. Reads `process.env` raw
   rather than injecting `ConfigService` (matching `data-source.ts`'s
   existing `process.env.DATABASE_URL` precedent for TypeORM) — simpler
   `useFactory` signature (no `inject: [ConfigService]`), and
   `ConfigModule.forRoot()` already populated `process.env` via dotenv
   before any other module's factory runs.

`isGlobal: true` (not per-module `register()` on `AppModule` alone) —
matches how `ConfigModule` is already wired in `init`'s `app.module.ts`
template, so caching is reachable the same way config already is,
without every future module needing its own import.

No example-usage file (matches `validation`'s DTO getting dropped too) —
the injection pattern (`@Inject(CACHE_MANAGER) private cacheManager:
Cache`) only makes sense inside a real service, and this tool doesn't
generate throwaway services.

Deliberately **not** adding `REDIS_URL` to `src/config/env.validation.ts`'s
zod schema — that needs a third patch shape (insert into a
`z.object({...})` call), and unlike `DATABASE_URL` (which every ORM's
provider fails hard without), a missing `REDIS_URL` just falls back to
`redis://localhost:6379` at the `useFactory` call site above, so it's
not load-bearing enough to justify the extra patch primitive.

---

## `ci add schedule`

Per [the docs](https://docs.nestjs.com/techniques/task-scheduling).

**Dependencies:** `@nestjs/schedule`. Nothing else — `@Cron()`/
`@Interval()`/`@Timeout()` and `SchedulerRegistry` all ship inside it.

**`app.module.ts` patch** — one import, one array entry, both idempotent
on `"ScheduleModule"`/`"ScheduleModule.forRoot"`:

```typescript
import { ScheduleModule } from '@nestjs/schedule';
```

```typescript
ScheduleModule.forRoot(),
```

No `.env` changes — nothing about task scheduling is environment-driven
(no external service to point at, unlike cache/queue's Redis).

**No example `@Cron()` handler.** Consistent with dropping validation's
DTO and cache's usage example: a cron job only makes sense attached to a
real piece of business logic, which this tool has no basis to invent.

---

## `ci add queue`

Per [the docs](https://docs.nestjs.com/techniques/queues), which cover
`@nestjs/bullmq` (BullMQ) — the docs' recommended option over the older
`@nestjs/bull`/Bull package, and the one that matches `cache`'s
already-Redis-backed precedent.

**Dependencies:** `@nestjs/bullmq`, `bullmq`, `ioredis`. `ioredis` is
listed explicitly (not left as a transitive dependency of `bullmq`)
because the module config below constructs an `IORedis` instance
directly — verified against
[BullMQ's own connection docs](https://docs.bullmq.io/guide/connections):
`QueueOptions.connection` accepts only a `{ host, port }` object or an
actual `ioredis` `Redis` instance, no raw connection-string field.

**`.env.example`/`.env`** — same `REDIS_URL` line `cache` already
appends, via the same `append_line` idempotency marker (`"REDIS_URL"`),
so running both `ci add cache` and `ci add queue` doesn't produce two
`REDIS_URL=` lines.

**`app.module.ts` patch** — two imports, one array entry, idempotent on
`"BullModule"`/`"BullModule.forRoot"`:

```typescript
import { BullModule } from '@nestjs/bullmq';
import IORedis from 'ioredis';
```

```typescript
BullModule.forRoot({
  connection: new IORedis(process.env.REDIS_URL ?? 'redis://localhost:6379', {
    maxRetriesPerRequest: null,
  }),
}),
```

`maxRetriesPerRequest: null` is required per BullMQ's docs whenever a
shared `ioredis` instance is handed to it — Workers open additional
internal blocking-command connections duplicated from this one, and
those duplicates need unlimited retries or BullMQ's internal retry logic
breaks.

**No named-queue registration, no processor/consumer scaffolding.**
`BullModule.registerQueue({ name: '...' })` and a `WorkerHost` consumer
are both inherently business-specific — same boundary `schedule` and
`cache` already draw.

---

## `ci add logger`

See the full "shipped in v0.1.2" section above — spec and rationale are
recorded there in one place rather than duplicated here.

---

## Rust implementation shape

```
src/commands/add/
  mod.rs             # dispatch: match Command::Validation/Cache/Schedule/Queue/Logger
  args.rs            # Args { command: Command };
                       # Command::{Validation, Cache, Schedule, Queue, Logger}
  listeners.rs        # PrintAction + HistoryAction wiring, identical to init/db's
  patch.rs            # detect_package_manager, install_dependencies,
                       # insert_after, insert_before, insert_after_last_import,
                       # insert_into_array, append_line, write_file_if_absent
  patch/tests.rs
  validation/
    mod.rs            # run(): install_dependencies + two main.ts inserts
    tests.rs
  cache/
    mod.rs            # run(): install_dependencies + .env append +
                       # two app.module.ts inserts
    tests.rs
  schedule/
    mod.rs            # run(): install_dependencies + two app.module.ts
                       # inserts, no .env change
    tests.rs
  queue/
    mod.rs            # run(): install_dependencies + .env append
                       # (shares cache's REDIS_URL marker) + two
                       # app.module.ts inserts
    tests.rs
  logger/
    mod.rs            # run(): write_file_if_absent x2 + two app.module.ts
                       # inserts + two main.ts inserts, no install
    tests.rs

src/shared/
  history/
    mod.rs             # HistoryAction: an Action that appends Finished/
                        # Error outcomes to <root>/ci/history.jsonl
    tests.rs
```

Template files under `templates/add/` — only `logger/` has any (`logger.
service.ts`/`logger.module.ts`, loaded via `include_str!` like every
other `.ts` template in this codebase); the other four subcommands only
ever patch existing files or shell out.

Command tags for the event bus (matching `db`'s specific-tag precedent,
and now also the keys `ci/history.jsonl` records under): `"init"`,
`"db migrate fresh"` (etc.), `"add validation"`, `"add cache"`,
`"add schedule"`, `"add queue"`, `"add logger"`.

No `detect.rs` equivalent needed — `add` doesn't branch on ORM, just
needs `main.ts`/`app.module.ts` to exist (which `patch.rs`'s "not found"
errors already cover without a separate detection pass).
`detect_package_manager` is a *different* kind of detection — advisory
(which installer to shell out to), not something worth failing a whole
run over, so it falls back to npm instead of erroring.

`src/args/mod.rs` gains `Commands::Add(add::Args)`; `src/commands/mod.rs`
gains the matching dispatch arm — same two-line addition `db` needed when
it was wired in.

---

## Testing

Same `InMemoryFileSystem` + `NoopCommandRunner` + `RecordingUi` pattern
every other command uses, but tests need to seed the fs with content
matching what `init` actually generates (not empty files) — `patch.rs`
operates on *existing* content, so a test with no `main.ts` present is
testing the "not found" error path, not the real one. Pull the seed
content from `templates::starter_files(...)`'s own output rather than a
hand-maintained fixture string that can silently stop matching what
`init` really produces.

Idempotency gets its own test per patch: run a subcommand's `run` twice
against the same `Context`, assert the second run doesn't duplicate
anything (and reports "already configured" rather than erroring).
`write_file_if_absent`-backed subcommands (`logger`) get an extra test:
hand-edit the generated file between runs, confirm the edit survives.

Multi-subcommand ordering gets its own test wherever two subcommands
touch the same file: `cache` then `schedule` (or either order) on
`app.module.ts`; `validation` then `logger` on `main.ts`, asserting the
full `NestFactory.create` → pipe → `useLogger` → `listen` order.

`shared::history::tests` covers `HistoryAction` directly against
`InMemoryFileSystem` — no project-template fixtures needed there, since
it only cares about `Event`s, not TypeScript content.

---

## Suggested build order

1. `patch.rs` — `detect_package_manager`, `install_dependencies`,
   `insert_after`, `insert_into_array`, each independently unit-testable.
2. `ci add validation` — the simpler of the first two patches, proves the
   pattern.
3. `ci add cache` — the array-insertion case.
4. Wire `Commands::Add` into `args`/`commands::mod`.
5. Integration tests seeded from real `templates::starter_files(...)`
   output, plus the idempotency tests.
6. `insert_after_last_import` — added once designing `queue` surfaced the
   fixed-anchor ordering bug; retrofit `cache`'s import insertion onto it
   at the same time.
7. `ci add schedule`, then `ci add queue` — reusing `cache`'s `REDIS_URL`
   marker; BullMQ's `connection: new IORedis(...)` shape confirmed
   against BullMQ's own docs first.
8. Integration + idempotency + ordering tests for both.
9. `ci add logger` — `write_file_if_absent` and `insert_before` added
   alongside it; `validation`'s `main.ts` import insertion retrofitted
   onto `insert_after_last_import` in the same change, for the same
   reason `cache`'s was retrofitted in step 6.
10. `ci/history.jsonl` — `HistoryAction` added to `shared::history`,
    wired into `add`/`db`/`init`'s `listeners::bus` (each now takes
    `root`); `init::run` restructured to compute `root` before its event
    lifecycle starts, so history has somewhere to write even when later
    steps fail.

---

## Explicitly not building yet

- **`--store <name>` to pick a non-Redis cache store.** Redis is the
  approved default; an in-memory or multi-store option behind a flag is
  a plausible follow-up, not built now.
- **An example DTO / cache-usage file / `@Cron()` handler / named-queue
  registration & consumer scaffolding / example log call.** All planned
  at some point across every `add` subcommand, all cut before shipping —
  each is inherently business-specific and this tool has no `generate
  controller`/`generate service` command yet to attach them to
  non-arbitrarily. Revisit once one exists.
- **A `--force` flag to re-patch even when the idempotency marker is
  found.** "Already configured, did nothing" seems like the right
  default for re-running an `add` subcommand by habit, not a footgun to
  design around yet.
- **`@nestjs/bull` (Bull, not BullMQ).** Went with BullMQ only, matching
  its status as the actively maintained option.
- **`NestFactory.create(AppModule, { bufferLogs: true })`.** `ci add
  logger`'s gap, explained in its section above — needs an "edit an
  existing line" patch primitive this codebase doesn't have yet (every
  primitive today only inserts).
- **`LOG_LEVEL` env var / non-console loggers (Winston, Pino) / per-context
  log formatting.** `ci add logger`'s default is deliberately just
  `ConsoleLogger`, unconfigured — same "don't build the flag until
  there's a second option" reasoning as `--store` above.
- **A `ci history` command to view/query `ci/history.jsonl`.** The file
  itself (newline-delimited JSON, one record per line) is already easy
  to `cat`/`grep`/pipe into `jq` by hand; a dedicated viewer is a
  plausible follow-up once someone actually wants filtering/formatting
  `jq` can't do trivially, not built now.
- **History retention/rotation/redaction.** The file grows forever and
  `message` can contain whatever an error's `Display` output happened to
  include (e.g. a full filesystem path, as seen in the verification run
  above) — fine for a local per-project log, but worth revisiting if this
  ever becomes something committed or shared rather than gitignored.
- **More `ci add` subcommands** (swagger, throttling, health checks,
  ...). The command's *shape* supports growth (one folder per
  subcommand, same as `db`); only building the five named/settled on so
  far.
