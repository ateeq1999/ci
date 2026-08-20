# Plan: `ci add` — wire NestJS techniques into an existing project

## Status: `validation`/`cache`/`schedule`/`queue`/`logger`/`events` all shipped in v0.1.2, plus cross-cutting `ci/history.jsonl` command history

## Note: `init` moved to Fastify and now bakes the logger in by default

Not part of this plan's own work, but load-bearing context for everything
below it: independently of the `ci add` work this document tracks,
`init`'s own templates changed —

- **Express → Fastify.** `templates/init/package.json` now depends on
  `@nestjs/platform-fastify`/`@fastify/cookie` instead of
  `@nestjs/platform-express`/`@types/express`; `main.ts` now does
  `NestFactory.create<NestFastifyApplication>(AppModule, new
  FastifyAdapter())` and registers `fastifyCookie`.
- **Every new project gets a logger out of the box**, not just projects
  that ran `ci add logger`: `commands::init::templates::FILES` now
  includes the *same* `templates/add/logger/logger.service.ts`/
  `logger.module.ts` files `ci add logger` writes, and `init`'s own
  `app.module.ts`/`main.ts` templates wire them up directly —
  `LoggerModule` in the imports array, `app.useLogger(app.get(AppLogger))`
  called right after `NestFactory.create`, before `fastifyCookie`
  registration and `app.listen`.

**What this means for `ci add logger`:** it still exists, still works,
and is now effectively a **backfill/repair** operation rather than an
opt-in one — running it against a project scaffolded by the current
`init` immediately reports "already configured" (every idempotency
marker it checks is already satisfied), verified directly. It remains
meaningful for projects scaffolded by an older `ci` version, or ones
where `src/logger/` was deleted by hand. Nothing about `ci add logger`'s
own code needed to change for this — same reason `ci add cache`/
`schedule`/`queue`/`events` don't need to change if their own techniques
were ever similarly promoted to an `init` default later.

**What this means for anchors:** `main.ts` no longer has the exact line
`"const app = await NestFactory.create(AppModule);"` `ci add validation`
originally anchored its pipe insertion on — the generic type parameter
and `FastifyAdapter` argument changed that line's exact text. Fixed (by
the same change) by shortening `validation`'s anchor to the substring
`"NestFactory.create"`, which still matches via `insert_after`'s
`l.contains(anchor)` regardless of what's inside the parens or angle
brackets. Every other `add` subcommand's anchors (`await app.listen(`,
`imports: [`, `import `-prefixed lines) were already loose enough not to
need any change.

**Why `ci add events` wasn't also baked into `init`.** Unlike a logger
(near-universally wanted, low-risk to have active and unused), an
in-process event emitter is a genuinely opt-in architectural choice — not
every project wants pub/sub-style decoupling internally, and registering
`EventEmitterModule.forRoot()` unconditionally would be a bigger,
unrequested default to impose than this plan's job covers. Stayed a pure
`ci add` subcommand, same as `validation`/`cache`/`schedule`/`queue`.

---

## `ci add events`: shipped in v0.1.2

Per [the docs](https://docs.nestjs.com/techniques/events), which cover
`@nestjs/event-emitter` — a thin wrapper (`eventemitter2` underneath) for
in-process pub/sub: `EventEmitter2.emit(...)` to fire an event,
`@OnEvent(...)` decorated methods to react to one.

**Dependencies: `@nestjs/event-emitter` only.** Confirmed via `npm view
@nestjs/event-emitter dependencies` before writing this: `eventemitter2`
is a real (non-peer) dependency of the Nest wrapper package, so it
arrives transitively — no need to list it in `install_dependencies`
separately, same shape as `class-validator`+`class-transformer` both
needing to be listed explicitly (peers of `@nestjs/common`, not of each
other) versus this case (one package pulls the other in on its own).

**`app.module.ts` patch** — one import, one array entry, both idempotent
(`"import { EventEmitterModule }"` for the import,
`"EventEmitterModule.forRoot"` — not the looser `"EventEmitterModule"` —
for the array entry, same "the import line's own text would falsely
satisfy the marker" trap `cache`/`schedule`/`logger` already avoid):

```typescript
import { EventEmitterModule } from '@nestjs/event-emitter';
```

```typescript
EventEmitterModule.forRoot(),
```

Bare `.forRoot()`, no config object — the docs show `wildcard`/
`delimiter`/`maxListeners`/etc. as available options, but defaults are
sane and there's no second concrete option to choose between yet (same
"don't build the flag until something needs it" reasoning already
applied to cache's `--store` and logger's `LOG_LEVEL`).

**No `.env` changes** — nothing about the event emitter is
environment-driven, matching `schedule`.

**No example emitted event, event class, or `@OnEvent()` listener.**
Same boundary every `add` subcommand already draws: what a project
actually emits/listens for is business-specific; `ci add events`'s job
ends at making `EventEmitter2` injectable and `@OnEvent()` usable.

No new patch primitives — reuses `install_dependencies`,
`insert_after_last_import`, `insert_into_array`, the exact same shape
`ci add schedule` already uses (one dependency, one import, one bare
`.forRoot()` array entry, no `.env`).

Verified: `src/commands/add/events/tests.rs` (installs the right package,
patches `app.module.ts` correctly against real `init` output including
the now-baked-in `LoggerModule` import already present; idempotency;
configured-package-manager; a stacking test running `schedule` then
`events` and asserting import order), plus a real `ci init` (Fastify +
baked-in logger) → `ci add events` run against the actual npm registry —
confirmed the real resolved version (`@nestjs/event-emitter ^3.1.0`,
matching the `npm view` check above), correct `app.module.ts` output
stacking after the pre-existing `LoggerModule` import, a clean second run
reporting "already configured" with no duplication, and both runs
recorded correctly in `ci/history.jsonl`.

---

## `ci add logger`: shipped in v0.1.2

Per [the docs](https://docs.nestjs.com/techniques/logger). **Default
logger is console** (per the ask) — NestJS's own built-in `ConsoleLogger`,
not Winston/Pino/any external logging library.

*Now also the `init` default — see the note at the top of this document.
Everything below describes the subcommand as built; it still works
exactly as described, just against a smaller set of projects that don't
already have it.*

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
`schedule`/`events` already avoid with `"CacheModule.registerAsync"`/
`"ScheduleModule.forRoot"`/`"EventEmitterModule.forRoot"`):

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
two can never collide or reorder each other no matter which runs first.
(`init`'s own baked-in logger wiring, added later, independently landed
its `useLogger` call in the *high* position instead — right after
`NestFactory.create`, before `fastifyCookie` registration — since it's
written directly into the template rather than patched in relative to
`app.listen`; no conflict either way, since the marker check is just "is
`app.useLogger` anywhere in the file," not "is it at a specific line.")

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
`append_line`) only ever inserts, none rewrites a line in place.

**No example log call, no `LOG_LEVEL` env var, no per-context
configuration.** Same boundary every other `add` subcommand already
draws.

**Writing genuinely new files needs its own idempotency shape.**
`logger` is the first `add` subcommand (and, since `init` also reuses its
templates now, the first *any* command) that creates brand-new files
rather than only patching existing ones:

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
`logger.service.ts` since it was first generated (by `init` or by
`ci add logger` itself). Verified directly with a test that hand-edits
the file, re-runs, and asserts the hand edit survives untouched.

Naming: **`logger`, not `log`.** `log` reads as a verb/action (matching
neither the docs' title, "Logger," nor this family's established pattern
of naming the *artifact* being wired in). `logger` also matches the class
this subcommand actually creates, `LoggerModule`.

---

## `schedule`/`queue`: shipped in v0.1.2

`insert_after_last_import` added to `patch.rs` first (the ordering-bug
fix), with `cache`'s import insertion retrofitted onto it in the same
change; `schedule` and `queue` then built on top, each its own
self-contained folder (`mod.rs`+`tests.rs`), matching `validation`/
`cache`.

Verified with a real end-to-end run: `ci init` → `ci add cache` → `ci add
schedule` → `ci add queue` against the actual npm registry, confirming
real resolved versions (`@nestjs/schedule ^6.1.3`, `@nestjs/bullmq
^11.0.5`, `bullmq ^6.1.2`, `ioredis ^6.0.0`), a single `REDIS_URL=` line
in `.env`/`.env.example` shared correctly between `cache` and `queue`,
imports landing in run order rather than colliding on one fixed anchor,
and re-running all three a second time reporting "already configured"
with zero duplication.

## `validation`/`cache`: shipped in v0.1.2

Built as designed below, then adjusted post-ship per follow-up feedback —
**`ci add caching` renamed to `ci add cache`**, **the example DTO
dropped** from `ci add validation`, and **dependency installation
replaced**: `patch.rs`'s original `add_dependencies` (parse
`package.json`, merge in a hand-guessed semver range) is gone, replaced
by `install_dependencies` + `detect_package_manager` — shells out to the
project's real package manager so it resolves genuine current versions.
Which package manager to use is read from `ci/config.json`'s
`packageManager` field, falling back to npm if the file's missing or
unparseable. `PackageManager` moved from `commands::init::args` to
`shared::package_manager` so both `init` and `add` share one definition.

Redis is `ci add cache`'s default store as approved —
`CacheModule.registerAsync` reading `process.env.REDIS_URL`,
`@keyv/redis` installed alongside `@nestjs/cache-manager`/
`cache-manager`, `REDIS_URL=redis://localhost:6379` appended to
`.env`/`.env.example`.

Supersedes the previous version of this file (the event-driven `Ui`/
`EventBus` plan — shipped, see `src/shared/events.rs`/`src/shared/ui.rs`
and every command's `listeners.rs`; this doc's job there is done — and
now the seam that `ci/history.jsonl` below hangs off).

## Goal

A new top-level command, `ci add <technique>`, for wiring a NestJS
technique into a project `ci init` already created:

```
ci add validation   # https://docs.nestjs.com/techniques/validation      — shipped
ci add cache        # https://docs.nestjs.com/techniques/caching         — shipped
ci add schedule     # https://docs.nestjs.com/techniques/task-scheduling — shipped
ci add queue        # https://docs.nestjs.com/techniques/queues          — shipped
ci add logger       # https://docs.nestjs.com/techniques/logger          — shipped (also now an init default)
ci add events       # https://docs.nestjs.com/techniques/events          — shipped
```

`add` is deliberately named for growth — same role `db` plays for
database operations, but for "wire technique X into this project."
Future candidates (not built now): `ci add swagger`, `ci add throttling`,
`ci add health-check`. Each becomes its own self-contained folder under
`src/commands/add/`, the same shape `db`'s five subcommands already use.

### Naming: `schedule`/`queue`/`logger`/`events`, not `task`/`crons`/`queues`/`log`/`event`

The ask floated `ci add task`, `ci add crons`, `ci add queues` for the
first pair, and `ci add logger`/`ci add log` for the third. Went with the
following, matching this project's own precedent (`caching` → `cache`
after the first round of feedback — singular, short, matches the npm
package name, and always names the *artifact*, never an action):

- **`schedule`, not `task` or `crons`.** NestJS's own doc title and npm
  package are both "Task Scheduling" / `@nestjs/schedule` — `schedule`
  matches the package name exactly.
- **`queue`, not `queues`.** Singular, matching `cache`/`db`/`add`
  themselves — the *subcommand* wires up one thing: queue infrastructure
  for the project.
- **`logger`, not `log`.** `log` reads as a verb/action; `logger` matches
  both the docs' title ("Logger") and the class this subcommand actually
  creates, `LoggerModule`.
- **`events`, not `event`.** The one break from "always singular" in this
  family, deliberately: the docs' own title is "Events" (plural), the npm
  package is `@nestjs/event-emitter`, and unlike `cache`/`queue` —
  already-natural general/mass nouns on their own ("a cache," "a queue")
  — `event` alone reads as *one specific occurrence*, not a capability
  name; a user typing `ci add event` would reasonably expect it to
  scaffold one particular event, not turn events on project-wide.
  `events` doesn't have that ambiguity and matches the docs exactly.
  (`event-emitter`, matching the npm package name exactly the way
  `schedule` does, was the other real candidate — passed over only for
  length; `events` says the same thing shorter.)

---

## The real problem: `ci` has never edited an existing file

Every command so far only ever *writes* files — `init` renders fresh
templates, `db`'s destructive operations shell out to other tools. Not
one line of this codebase modifies a file that's already there.
`ci add validation` needs to edit `src/main.ts`; `ci add cache` needs to
edit `src/app.module.ts`. Both need `package.json` updated with new
dependencies. This is new capability, not a mechanical reuse of what
`init`/`db` already do — worth its own section before the subcommands,
since all of them depend on it.

### Dependencies: shell out to the real package manager, don't guess versions

Shell out to whichever package manager the project is configured for
(`npm install <pkgs>` — npm has no separate `add` verb — or `pnpm add`/
`yarn add`), and let *it* resolve and write real current versions into
`package.json`. Confirmed better in practice across every subcommand
built so far: real runs resolved `class-validator ^0.15.1`,
`@nestjs/cache-manager ^3.1.3`, `@nestjs/schedule ^6.1.3`, `@nestjs/bullmq
^11.0.5`, `@nestjs/event-emitter ^3.1.0` — all newer than this plan's
original guesses. `ci add logger` is the one exception: it needs no
dependencies at all, so it never calls `install_dependencies`.
`ci add events` needed one extra check before shipping: confirming
`eventemitter2` (the library `@nestjs/event-emitter` wraps) is a *real*
dependency of that package, not a peer — `npm view @nestjs/event-emitter
dependencies` before writing any code, so it doesn't need listing
separately.

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

### `main.ts`/`app.module.ts`: anchor-text insertion, not a TS parser

No TS AST tooling in Rust worth pulling in for a handful of insertion
points. `ci` already knows *roughly* what these files look like — it
wrote them (or `init` did) from a known template. Anchor on a known
substring, not a general parse — loose enough to survive `main.ts`
changing shape later (as it did, independently, for the Fastify switch —
see the note at the top of this document):

```rust
// src/commands/add/patch.rs
pub fn insert_after(
    ctx: &Context,
    path: &Path,
    anchor: &str,
    already_present_marker: &str,
    line: &str,
) -> Result<bool> { /* ... */ }
```

Returns `Ok(false)` for "already there" rather than erroring.

### Multiple subcommands patching the same file: anchor on the *last* import, not a fixed line

Found while designing `ci add queue`: `cache` inserted its import line
right after one fixed anchor (`DatabaseModule`'s import). If `schedule`
and `queue` did the same, every subcommand's import would land at that
*same* fixed position — each new `add` run landing ahead of whatever a
previous run had already put there. Fix: a primitive that anchors
dynamically on whichever import line is currently *last*:

```rust
// src/commands/add/patch.rs
pub fn insert_after_last_import(
    ctx: &Context,
    path: &Path,
    already_present_marker: &str,
    lines: &str,
) -> Result<bool> { /* rposition on l.trim_start().starts_with("import "), insert after it */ }
```

Retrofitted onto every subcommand that patches `app.module.ts`
(`cache`/`schedule`/`queue`/`logger`/`events`) and both subcommands that
patch `main.ts` (`validation`/`logger`) — it's now *the* way any `add`
subcommand adds an import line anywhere, full stop.

`insert_into_array` needs no equivalent treatment — array insertion
already targets "right after `[`," inherently order-stable since array
element order doesn't carry the same "reads top-to-bottom" expectation
import statements do.

### Statements that must stay *last*, not first: `insert_before`

`ci add logger`'s `app.useLogger(...)` call needs to run somewhere
between `NestFactory.create` and `app.listen`. Rather than build a second
dynamic "last statement" tracker, it anchors on `await app.listen(` — a
distinct, stable anchor no other subcommand touches — and `insert_before`
inserts directly above it:

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
other, the two can't collide regardless of which subcommand runs first.

### Writing brand-new files: `write_file_if_absent`

`ci add logger` needed a primitive for "does this whole file already
exist" (`init` later reused the same underlying template files, for the
same reason):

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

---

## `ci add validation`

Per [the docs](https://docs.nestjs.com/techniques/validation).
**Dependencies:** `class-validator`, `class-transformer`.

**`main.ts` patch** — insert right after the `NestFactory.create` line:

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
`insert_after_last_import`.

**No example DTO.** Nothing wires it into a route (this tool has no
`generate controller` to attach it to non-arbitrarily), so it would just
be inert dead code. `ci add validation`'s whole job ends at making the
pipe actually active.

---

## `ci add cache`

Per [the docs](https://docs.nestjs.com/techniques/caching).
**Redis is the default store** (approved) — not the bare in-memory
`CacheModule.register()` the docs lead with.

**Dependencies:** `@nestjs/cache-manager`, `cache-manager`, `@keyv/redis`.

**`.env.example`/`.env` gets a new line:**

```
REDIS_URL=redis://localhost:6379
```

**`app.module.ts` patch:**

```typescript
CacheModule.registerAsync({
  isGlobal: true,
  useFactory: async () => ({
    stores: [new KeyvRedis(process.env.REDIS_URL ?? 'redis://localhost:6379')],
  }),
}),
```

`registerAsync`+`useFactory`, not the synchronous `register()` the docs
lead with — needed once the store depends on an env var read at startup.

No example-usage file. Deliberately not adding `REDIS_URL` to the zod env
schema — a missing value just falls back to `localhost:6379` at the
`useFactory` call site, not load-bearing enough to justify a third patch
shape.

---

## `ci add schedule`

Per [the docs](https://docs.nestjs.com/techniques/task-scheduling).
**Dependencies:** `@nestjs/schedule` — `@Cron()`/`@Interval()`/
`@Timeout()`/`SchedulerRegistry` all ship inside it.

```typescript
import { ScheduleModule } from '@nestjs/schedule';
```
```typescript
ScheduleModule.forRoot(),
```

No `.env` changes. No example `@Cron()` handler.

---

## `ci add queue`

Per [the docs](https://docs.nestjs.com/techniques/queues), covering
`@nestjs/bullmq` (BullMQ) over the older `@nestjs/bull`.

**Dependencies:** `@nestjs/bullmq`, `bullmq`, `ioredis` — `ioredis` listed
explicitly since `QueueOptions.connection` (per
[BullMQ's own docs](https://docs.bullmq.io/guide/connections)) accepts
only a `{ host, port }` object or an actual `ioredis` instance, no raw
connection string.

**`.env.example`/`.env`** — same `REDIS_URL` line `cache` appends, shared
idempotency marker so running both doesn't duplicate it.

```typescript
BullModule.forRoot({
  connection: new IORedis(process.env.REDIS_URL ?? 'redis://localhost:6379', {
    maxRetriesPerRequest: null,
  }),
}),
```

`maxRetriesPerRequest: null` required per BullMQ's docs whenever a shared
`ioredis` instance is handed to it. No named-queue registration, no
processor/consumer scaffolding.

---

## `ci add logger`

See the full "shipped in v0.1.2" section above.

## `ci add events`

See the full "shipped in v0.1.2" section above.

---

## Rust implementation shape

```
src/commands/add/
  mod.rs             # dispatch: match Command::{Validation,Cache,Schedule,Queue,Logger,Events}
  args.rs            # Args { command: Command }
  listeners.rs        # PrintAction + HistoryAction wiring, identical to init/db's
  patch.rs            # detect_package_manager, install_dependencies,
                       # insert_after, insert_before, insert_after_last_import,
                       # insert_into_array, append_line, write_file_if_absent
  patch/tests.rs
  validation/  { mod.rs, tests.rs }
  cache/       { mod.rs, tests.rs }
  schedule/    { mod.rs, tests.rs }
  queue/       { mod.rs, tests.rs }
  logger/      { mod.rs, tests.rs }
  events/      { mod.rs, tests.rs }  # run(): install_dependencies + two
                                      # app.module.ts inserts, no .env
                                      # change — same shape as schedule

src/shared/
  history/ { mod.rs, tests.rs }  # HistoryAction: appends Finished/Error
                                   # outcomes to <root>/ci/history.jsonl
```

Template files under `templates/add/` — only `logger/` has any
(`logger.service.ts`/`logger.module.ts`, now also reused directly by
`init`'s own templates — see the note at the top of this document); every
other subcommand only ever patches existing files or shells out.

Command tags (matching `db`'s specific-tag precedent, and the keys
`ci/history.jsonl` records under): `"init"`, `"db migrate fresh"` (etc.),
`"add validation"`, `"add cache"`, `"add schedule"`, `"add queue"`,
`"add logger"`, `"add events"`.

`src/args/mod.rs` gains `Commands::Add(add::Args)`; `src/commands/mod.rs`
gains the matching dispatch arm.

---

## Testing

Same `InMemoryFileSystem` + `NoopCommandRunner` + `RecordingUi` pattern
every other command uses, seeded from real `templates::starter_files(...)`
output rather than hand-maintained fixtures — this is what kept every
`add` subcommand's tests green through the Fastify/baked-in-logger
change without any test needing to change (they read the *current* real
template each run, not a frozen copy).

Idempotency gets its own test per patch. `write_file_if_absent`-backed
subcommands (`logger`) get an extra test: hand-edit the generated file
between runs, confirm the edit survives. Multi-subcommand ordering gets
its own test wherever two subcommands touch the same file (`cache` then
`schedule`; `validation` then `logger`; `schedule` then `events`).

`shared::history::tests` covers `HistoryAction` directly against
`InMemoryFileSystem` — no project-template fixtures needed there, since
it only cares about `Event`s, not TypeScript content.

---

## Suggested build order

1. `patch.rs` core primitives (`detect_package_manager`,
   `install_dependencies`, `insert_after`, `insert_into_array`), each
   independently unit-testable.
2. `ci add validation`, then `ci add cache` — prove the pattern, then the
   array-insertion case.
3. Wire `Commands::Add` into `args`/`commands::mod`; integration +
   idempotency tests.
4. `insert_after_last_import` — added once designing `queue` surfaced the
   fixed-anchor ordering bug; retrofit `cache`'s import insertion onto it.
5. `ci add schedule`, then `ci add queue`.
6. `ci add logger` — `write_file_if_absent` and `insert_before` added
   alongside it; `validation`'s `main.ts` import insertion retrofitted
   onto `insert_after_last_import` in the same change.
7. `ci/history.jsonl` — `HistoryAction` added to `shared::history`, wired
   into `add`/`db`/`init`'s `listeners::bus` (each now takes `root`);
   `init::run` restructured to compute `root` before its event lifecycle
   starts.
8. *(Independent of this plan)* `init` moved to Fastify and started
   reusing `templates/add/logger/*` directly — see the note at the top of
   this document.
9. `ci add events` — reuses every existing primitive, no new ones;
   confirmed `eventemitter2` is a real (not peer) dependency of
   `@nestjs/event-emitter` via `npm view` before writing any code.

---

## Explicitly not building yet

- **`--store <name>` to pick a non-Redis cache store**, **`LOG_LEVEL` env
  var / non-console loggers**, **`EventEmitterModule.forRoot()` config
  options (`wildcard`, `delimiter`, ...).** Same reasoning across all
  three: defaults are sane, no second concrete option exists yet to
  justify the flag.
- **An example DTO / cache-usage file / `@Cron()` handler / named-queue
  registration & consumer scaffolding / example log call / example
  emitted event or `@OnEvent()` listener.** All inherently
  business-specific; this tool has no `generate controller`/`generate
  service` command yet to attach them to non-arbitrarily.
- **A `--force` flag to re-patch even when the idempotency marker is
  found.** "Already configured, did nothing" is the right default.
- **`@nestjs/bull` (Bull, not BullMQ).**
- **`NestFactory.create(AppModule, { bufferLogs: true })`.** Needs an
  "edit an existing line" patch primitive this codebase doesn't have yet.
- **A `ci history` command to view/query `ci/history.jsonl`.** The file
  itself is already easy to `cat`/`grep`/pipe into `jq` by hand.
- **History retention/rotation/redaction.** Fine for a local, gitignored,
  per-project log; worth revisiting if it's ever committed/shared.
- **Baking `events` into `init` by default**, the way `logger` ended up.
  A deliberate call, not an oversight — see the note at the top of this
  document.
- **More `ci add` subcommands** (swagger, throttling, health checks,
  ...). The command's *shape* supports growth; only building the six
  named/settled on so far.
