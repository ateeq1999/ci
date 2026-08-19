# Plan: `ci add` — wire NestJS techniques into an existing project

## Status: shipped in v0.1.2

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
appended to `.env`/`.env.example`. Verified: 64 tests passing (`patch.rs`'s
primitives unit-tested against small fixtures — including
`detect_package_manager`'s npm-fallback cases and `install_dependencies`'s
per-manager verb selection — both subcommands integration-tested against
*real* `init` template output via `templates::starter_files(...)` rather
than hand-typed fixtures, an idempotency test per subcommand, and a
configured-package-manager test per subcommand), plus a real `ci init`
(pnpm-configured) → `npm install` → `ci add validation` → `ci add cache`
run against an actual npm registry, confirming real installed versions
(e.g. `class-validator ^0.15.1`, all newer than this plan's original
hardcoded guesses — direct evidence the shell-out approach was the right
call), correct `main.ts`/`app.module.ts`/`.env` patches, and no
`example.dto.ts` written.

Supersedes the previous version of this file (the event-driven `Ui`/
`EventBus` plan — shipped, see `src/shared/events.rs`/`src/shared/ui.rs`
and every command's `listeners.rs`; this doc's job there is done).

## Goal

A new top-level command, `ci add <technique>`, for wiring a NestJS
technique into a project `ci init` already created — starting with the
two from the ask:

```
ci add validation   # https://docs.nestjs.com/techniques/validation
ci add cache        # https://docs.nestjs.com/techniques/caching
```

`add` is deliberately named for growth — same role `db` plays for
database operations, but for "wire technique X into this project."
Future candidates (not built now): `ci add swagger`, `ci add throttling`,
`ci add health-check`. Each becomes its own self-contained folder under
`src/commands/add/`, the same shape `db`'s five subcommands already use.

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
before the two subcommands, since both depend on it.

### Dependencies: shell out to the real package manager, don't guess versions

Original draft of this section proposed parsing `package.json` as JSON
and merging in hand-picked semver ranges directly. Shipped differently,
per follow-up feedback: shell out to whichever package manager the
project is configured for (`npm install <pkgs>` — npm has no separate
`add` verb — or `pnpm add`/`yarn add`, which reserve `install` for
"install from the lockfile only"), and let *it* resolve and write real
current versions into `package.json`. Confirmed better in practice: a
real run resolved `class-validator ^0.15.1`, `@nestjs/cache-manager
^3.1.3` — both newer than this plan's original guesses.

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

No TS AST tooling in Rust worth pulling in for two insertion points. But
`ci` already knows *exactly* what these files look like — it wrote them
with `init`'s templates. Anchor on a known line from that template, not
a general parse:

```rust
// src/commands/add/patch.rs
/// Inserts `line` right after the first line containing `anchor`, unless
/// `line` (or anything containing `already_present_marker`) is already in
/// the file — idempotent, and errors clearly instead of guessing if the
/// anchor isn't found (e.g. someone hand-edited the file away from what
/// `init` generated).
pub fn insert_after(
    ctx: &Context,
    path: &Path,
    anchor: &str,
    already_present_marker: &str,
    line: &str,
) -> Result<bool> {
    let contents = ctx.fs.try_read_to_string(path)?
        .ok_or_else(|| anyhow!("{} not found", path.display()))?;
    if contents.contains(already_present_marker) {
        return Ok(false); // already wired up — nothing to do
    }
    let mut out = String::new();
    let mut inserted = false;
    for l in contents.lines() {
        out.push_str(l);
        out.push('\n');
        if !inserted && l.contains(anchor) {
            out.push_str(line);
            out.push('\n');
            inserted = true;
        }
    }
    if !inserted {
        bail!("couldn't find `{anchor}` in {} — has it been hand-edited?", path.display());
    }
    ctx.fs.write_file(path, &out)?;
    Ok(true)
}
```

Returns `Ok(false)` for "already there" rather than erroring — `ci add
validation` run twice should say "already configured," not fail.

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

Plus `import { ValidationPipe } from '@nestjs/common';` merged into the
existing `@nestjs/common` import (or a new import line — simpler to add a
second `import` line than to parse and merge named-import lists; a
duplicate named import from the same module isn't valid TS, so this needs
its own small check, not just `insert_after`'s "does this exact line
exist" idempotency).

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
`"CacheModule"`:
1. `import { CacheModule } from '@nestjs/cache-manager';` and
   `import { KeyvRedis } from '@keyv/redis';`
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

   This is a different insertion shape than `main.ts`'s "after this
   line" (needs "as the first/last element of this array"), so
   `patch.rs` needs a second helper, not just `insert_after`:

```rust
/// Inserts `item` as a new element right after the array's opening `[` on
/// the line containing `array_anchor` (e.g. `"imports: ["`) — same
/// idempotency-marker approach as `insert_after`.
pub fn insert_into_array(
    ctx: &Context,
    path: &Path,
    array_anchor: &str,
    already_present_marker: &str,
    item: &str,
) -> Result<bool> { /* ... */ }
```

`isGlobal: true` (not per-module `register()` on `AppModule` alone) —
matches how `ConfigModule` is already wired in `init`'s `app.module.ts`
template, so caching is reachable the same way config already is,
without every future module needing its own import.

No example-usage file (matches `validation`'s DTO getting dropped too) —
the injection pattern (`@Inject(CACHE_MANAGER) private cacheManager:
Cache`) only makes sense inside a real service, and this tool doesn't
generate throwaway services. `README.md`/doc-comment territory, not a
template file.

Deliberately **not** adding `REDIS_URL` to `src/config/env.validation.ts`'s
zod schema this pass — that needs a third patch shape (insert into a
`z.object({...})` call), and unlike `DATABASE_URL` (which every ORM's
provider fails hard without), a missing `REDIS_URL` just falls back to
`redis://localhost:6379` at the `useFactory` call site above, so it's
not load-bearing enough yet to justify the extra patch primitive.

---

## Rust implementation shape

```
src/commands/add/
  mod.rs             # dispatch: match Command::Validation/Cache
  args.rs            # Args { command: Command }; Command::{Validation, Cache}
  listeners.rs        # PrintAction wiring, identical to init/update/db's
  patch.rs            # detect_package_manager, install_dependencies,
                       # insert_after, insert_into_array, append_line
  patch/tests.rs
  validation/
    mod.rs            # run(): install_dependencies + two main.ts inserts
    tests.rs
  cache/
    mod.rs            # run(): install_dependencies + .env append +
                       # two app.module.ts inserts
    tests.rs
```

No template files under `templates/add/` — both subcommands only ever
patch existing files or shell out; there was going to be one (the
example DTO) but it got cut.

Command tags for the event bus (matching `db`'s specific-tag precedent):
`"add validation"`, `"add cache"`.

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
content from `templates::starter_files(...)`'s own output (call it in the
test, matching a fixture, instead of hand-typing a copy of `main.ts` that
can drift from the real template) rather than a hand-maintained fixture
string that can silently stop matching what `init` really produces.

Idempotency gets its own test per patch: run `add::validation::run` twice
against the same `Context`, assert the second run doesn't duplicate the
`ValidationPipe` block (and reports "already configured" rather than
erroring).

---

## Suggested build order

1. `patch.rs` — `detect_package_manager`, `install_dependencies`,
   `insert_after`, `insert_into_array`, each independently unit-testable
   against small hand-written fixture strings (not full
   `main.ts`/`app.module.ts` yet).
2. `ci add validation` — the simpler of the two patches (one array-less
   insertion point plus one dependency-only file), proves the pattern.
3. `ci add cache` — the array-insertion case.
4. Wire `Commands::Add` into `args`/`commands::mod`.
5. Integration tests seeded from real `templates::starter_files(...)`
   output, plus the idempotency tests.

---

## Explicitly not building yet

- **`--store <name>` to pick a non-Redis store.** Redis is the approved
  default; an in-memory or multi-store (`Keyv` + `KeyvCacheableMemory`
  layered with `KeyvRedis`, per the docs) option behind a flag is a
  plausible follow-up, not built now.
- **An example DTO / example cache-usage file.** Both got planned, both
  got cut before shipping — nothing wires them into a route or service,
  and this tool has no `generate controller`/`generate service` command
  yet to make that not-arbitrary. Revisit once one exists.
- **A `--force` flag to re-patch even when the idempotency marker is
  found.** Not clearly needed — "already configured, did nothing" seems
  like the right default behavior for re-running `ci add validation` by
  habit, not a footgun to design around yet.
- **More `ci add` subcommands** (swagger, throttling, health checks,
  ...). The command's *shape* supports growth (one folder per
  subcommand, same as `db`); only building the two named in the ask.
