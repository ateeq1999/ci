# Plan: `ci add` — wire NestJS techniques into an existing project

## Status: shipped in v0.1.2

Seven subcommands (`validation`, `cache`, `schedule`, `queue`, `logger`,
`events`, `compression`), plus cross-cutting `ci/history.jsonl` command
history.
`init` independently moved to Fastify and now bakes `logger` in by
default (see below) — unrelated to this plan but relevant context.

## Goal

`ci add <technique>` wires one NestJS technique into a project `ci init`
already created — same role `db` plays for database operations. Each
subcommand is a self-contained folder under `src/commands/add/`
(`mod.rs` + `tests.rs`), matching `db`'s shape. Future candidates (not
built): `swagger`, `throttling`, `health-check`.

```
ci add validation   # https://docs.nestjs.com/techniques/validation
ci add cache        # https://docs.nestjs.com/techniques/caching
ci add schedule     # https://docs.nestjs.com/techniques/task-scheduling
ci add queue        # https://docs.nestjs.com/techniques/queues
ci add logger       # https://docs.nestjs.com/techniques/logger (also an init default)
ci add events       # https://docs.nestjs.com/techniques/events
ci add compression  # https://docs.nestjs.com/techniques/compression
```

Naming always picks the artifact, never a verb: `cache` not `caching`,
`queue` not `queues`, `logger` not `log`, `compression` not `compress`.
`events` is the one plural — "event" alone reads as one occurrence, not
a capability, unlike `cache`/`queue`/`compression`, already mass nouns.

---

## How patching works

`ci` never had to edit an existing file before this — `init` only
renders fresh templates, `db` only shells out. `patch.rs` holds the
primitives every subcommand builds on, all anchor-text based (no TS
parser — not worth it for a handful of insertion points) and all
idempotent (`Ok(false)` if already present, not an error):

- **`detect_package_manager`** — reads `packageManager` from
  `ci/config.json`, falls back to npm.
- **`install_dependencies`** — shells out to the real package manager
  (`npm install`/`pnpm add`/`yarn add`) rather than guessing semver
  ranges; every real run has resolved newer versions than any guess
  would have.
- **`insert_after`** — insert after the first line containing an anchor.
- **`insert_after_last_import`** — insert after the *last* `import` line,
  not a fixed one. Needed once more than one subcommand adds an import to
  the same file (`app.module.ts`'s `cache`/`schedule`/`queue`/`logger`/
  `events`, `main.ts`'s `validation`/`logger`) — a fixed anchor would
  make every subsequent subcommand's import land in the same spot,
  ahead of whatever an earlier one inserted.
- **`insert_before`** — insert before an anchor, for content that must
  stay *last* (e.g. `logger`'s `app.useLogger(...)`, anchored on
  `await app.listen(`, so it can never collide with `validation`'s pipe
  regardless of run order).
- **`insert_into_array`** — insert as a new element after `[` on the
  line containing an anchor (e.g. `imports: [`). Naturally order-stable
  across subcommands — no dynamic-anchor version needed.
- **`append_line`** — append to `.env`/`.env.example`, idempotent.
- **`write_file_if_absent`** — write a brand-new file only if nothing's
  there yet (`logger`'s `logger.service.ts`/`logger.module.ts`); never
  clobbers a hand edit on re-run.

All anchors are loose substrings on purpose — they survived `main.ts`
changing shape (Express → Fastify) with zero code changes.

---

## Subcommands

**`validation`** — `class-validator`, `class-transformer`. Inserts
`app.useGlobalPipes(new ValidationPipe({ whitelist, forbidNonWhitelisted,
transform: true }))` after `NestFactory.create`. No example DTO — nothing
wires it into a route.

**`cache`** — `@nestjs/cache-manager`, `cache-manager`, `@keyv/redis`.
Redis is the default store (approved, not the docs' in-memory default).
`CacheModule.registerAsync({ isGlobal: true, useFactory: ... new
KeyvRedis(process.env.REDIS_URL ?? 'redis://localhost:6379') })`.
Appends `REDIS_URL=redis://localhost:6379` to `.env`/`.env.example`.

**`schedule`** — `@nestjs/schedule` only. `ScheduleModule.forRoot()`.
No `.env`, no example `@Cron()` handler.

**`queue`** — `@nestjs/bullmq`, `bullmq`, `ioredis` (listed explicitly:
BullMQ's `connection` option takes only a `{host,port}` object or an
`ioredis` instance, no URL string — confirmed against BullMQ's own
docs). `BullModule.forRoot({ connection: new IORedis(REDIS_URL, {
maxRetriesPerRequest: null }) })` — that option is required whenever a
shared `ioredis` instance is handed to BullMQ. Shares `cache`'s
`REDIS_URL` env line/marker. No named-queue or consumer scaffolding.

**`logger`** — no dependencies (`ConsoleLogger` ships in
`@nestjs/common`). Writes a standalone `src/logger/` module matching
`DatabaseModule`'s `@Global()` shape: `AppLogger extends ConsoleLogger`,
`LoggerModule` providing/exporting it. Registers it in `app.module.ts`
and calls `app.useLogger(app.get(AppLogger))` in `main.ts` (inserted
before `app.listen`, not after `NestFactory.create`, so it can't collide
with `validation`'s pipe). No `bufferLogs: true` — that means editing an
existing line's arguments, a patch shape `ci` doesn't have.
**Now also `init`'s own default** (see below) — running it against a
fresh project reports "already configured"; it's a backfill for older
projects now.

**`events`** — `@nestjs/event-emitter` only (`eventemitter2` is a real,
not peer, dependency — confirmed via `npm view` before building this).
`EventEmitterModule.forRoot()`, no config object. No `.env`, no example
emitted event or `@OnEvent()` listener. Deliberately *not* baked into
`init` the way `logger` was — an event emitter is a real opt-in
architectural choice, not a near-universal default.

**`compression`** — `@fastify/compress` only (Fastify-only; `init` no
longer scaffolds Express). `await app.register(compression);` via
`insert_before` on `await app.listen(` — same anchor `logger`'s
`useLogger` uses, stacking after it rather than colliding (multiple
`insert_before` calls on one anchor naturally stack in run order). No
`app.module.ts` change — bootstrap middleware, not a module.

Every subcommand's idempotency marker for its array entry is the call
itself (e.g. `"CacheModule.registerAsync"`, `"ScheduleModule.forRoot"`),
never the bare module name — the bare name is also a substring of the
import line, which would make the array-insertion check pass falsely.

---

## `init`: Fastify + baked-in logger (independent of this plan)

`init`'s own templates now scaffold Fastify (`@nestjs/platform-fastify`,
`@fastify/cookie`) instead of Express, and reuse the *same*
`templates/add/logger/*.ts` files `ci add logger` writes, wiring
`LoggerModule`/`AppLogger` in by default. This changed `main.ts`'s exact
text, which is why `validation`'s pipe-insertion anchor was shortened
from the full `NestFactory.create(AppModule)` line to just
`"NestFactory.create"` (still matches via substring). No other
subcommand needed changes — every other anchor was already loose enough.

---

## `ci/history.jsonl` — command history

Every project-scoped command (`init`, `db`, `add`; not `update`, which
has no project root) appends one JSON line to `<root>/ci/history.jsonl`
on completion: `{"timestamp", "command", "status": "success"|"error",
"message"}`. Implemented as `HistoryAction` (`src/shared/history/`), a
second `Action` alongside `PrintAction` in every project-scoped
`listeners::bus` — the event system already had this seam reserved
(`Started`/`Updated`/`Warned`/`Finished`/`Error`; only the last two are
recorded). Best-effort: a write failure never fails the command itself.
Gitignored (`ci/history.jsonl`) — it's a local execution log, not shared
config like `ci/config.json`.

`init::run` validates `name` and computes `root` *before* building its
bus, so history has somewhere to write even when nothing else succeeds.

---

## Rust shape

```
src/commands/add/
  mod.rs, args.rs, listeners.rs, patch.rs (+ patch/tests.rs)
  validation/ cache/ schedule/ queue/ logger/ events/ compression/   (mod.rs + tests.rs each)

src/shared/history/   (HistoryAction: mod.rs + tests.rs)
```

Command tags double as history keys: `"add validation"`, `"add cache"`,
`"add schedule"`, `"add queue"`, `"add logger"`, `"add events"`,
`"add compression"`.

Tests seed `InMemoryFileSystem` from real `templates::starter_files(...)`
output, not hand-typed fixtures — this is what kept every subcommand's
tests passing through the Fastify/logger-default change with zero test
edits. Every subcommand gets an idempotency test (run twice, assert no
duplication) and, wherever two subcommands touch the same file, an
ordering test (`cache` then `schedule`; `validation` then `logger`;
`schedule` then `events`; `logger` then `compression`).

---

## Explicitly not building yet

- Config flags (`--store`, `LOG_LEVEL`, `EventEmitterModule` options,
  compression `encodings`) — defaults are sane, no second concrete
  option exists yet to justify one.
- Example DTOs, cache/log usage, `@Cron()` handlers, queue consumers,
  `@OnEvent()` listeners — business-specific; no `generate controller`/
  `generate service` yet to attach them to non-arbitrarily.
- `--force` to re-patch past the idempotency marker; `@nestjs/bull`
  (Bull, not BullMQ); `NestFactory.create(..., { bufferLogs: true })`
  (needs an "edit an existing line" primitive `ci` doesn't have).
- A `ci history` viewer command, and history retention/rotation/redaction.
- Baking `events`/`compression` into `init` by default.
- More `ci add` subcommands (swagger, throttling, health checks, ...).
