# Plan: `init` — scaffold a new NestJS project

Scope check: dropping React Native / TanStack Start for now. This tool is a
NestJS-only code generator, starting with `init` (create a new project),
per [First Steps](https://docs.nestjs.com/first-steps). The other links in
`nestjs-urls.txt` (controllers, providers, modules, middleware, exception
filters, pipes, guards, custom decorators) map to future `generate`
subcommands — noted as a roadmap at the bottom, not built now.

---

## Step 0 — Remove the demo scaffolding

Delete the tutorial leftovers so the command surface is only what this tool
actually does:

- `src/commands/dev.rs` — delete file.
- `src/commands/mod.rs` — drop the `dev` match arm and the `Run` /
  `None` → "Hello, {name}" arms; `run()` will dispatch real commands only.
- `src/args/mod.rs` — remove `Commands::Run` and `Commands::Dev`; remove the
  top-level `name` field on `Cli` (it only existed for the greeting demo).

**Checkpoint:** `cargo build` succeeds with an empty (or `todo!()`)
`Commands` enum.

---

## Step 1 — Decide how `init` creates the project

Three options, ranked:

1. **Embedded templates (recommended).** This tool ships its own minimal
   NestJS starter (package.json, tsconfig.json, nest-cli.json, src/main.ts,
   src/app.module.ts, src/app.controller.ts, src/app.service.ts) as files
   under `templates/nestjs-starter/`, embedded into the binary with
   `include_str!` or the `rust-embed` crate. `init` writes them out, then
   optionally runs `npm install` / `git init`.
   - No dependency on `@nestjs/cli` or network access at generate time.
   - Matches this tool's identity as a generator (same `FileSystem`
     abstraction will be reused by every future `generate` subcommand).
   - Cost: you own keeping the template in sync with NestJS's own starter.
2. **Clone the official starter.** Shell out to
   `git clone https://github.com/nestjs/typescript-starter.git <dir>`, then
   `npm install`. Less code, always up to date, but requires git + network,
   and doesn't build the reusable "write templated files" path the future
   `generate` commands need anyway.
3. **Wrap the Nest CLI.** Shell out to `nest new` / `npx @nestjs/cli new`.
   Least code, but adds close to zero value over the user just running Nest
   CLI directly — not recommended.

Going with **Option 1** since it reuses the `FileSystem`/template DI seam
this whole tool is built around, and every later `generate` subcommand
(controller, module, service, ...) needs that same machinery.

---

## Step 2 — Command surface (`args/mod.rs`)

```rust
#[derive(Subcommand)]
pub enum Commands {
    /// Create a new NestJS project
    Init {
        /// Project directory / name
        name: String,

        #[arg(long, value_enum, default_value = "npm")]
        package_manager: PackageManager,

        /// Write files without running `npm install`
        #[arg(long)]
        skip_install: bool,

        /// Don't run `git init` in the new project
        #[arg(long)]
        skip_git: bool,
    },
}

#[derive(Clone, ValueEnum)]
pub enum PackageManager {
    Npm,
    Pnpm,
    Yarn,
}
```

**Checkpoint:** `cargo run -- init my-api` parses; match arm can `todo!()`.

---

## Step 3 — `FileSystem` trait (`src/fs.rs`)

Same seam as the original plan — generators (starting with `init`) never
call `std::fs` directly, so they're testable without touching disk.

```rust
pub trait FileSystem {
    fn write_file(&self, path: &Path, contents: &str) -> anyhow::Result<()>;
}

pub struct RealFileSystem;   // std::fs, creates parent dirs
pub struct InMemoryFileSystem { .. }  // HashMap<PathBuf, String>, for tests
```

**Checkpoint:** unit test writes through `InMemoryFileSystem` and asserts
the map contents.

---

## Step 4 — Embed the starter template

- Add `rust-embed` (or plain `include_str!` if the file list is small and
  fixed) as a dependency.
- Create `templates/nestjs-starter/` with the minimal set of files a fresh
  `nest new` project has: `package.json`, `tsconfig.json`,
  `tsconfig.build.json`, `nest-cli.json`, `.eslintrc.js` (or a flat config),
  `.prettierrc`, `src/main.ts`, `src/app.module.ts`,
  `src/app.controller.ts`, `src/app.service.ts`, `src/app.controller.spec.ts`.
- `package.json` needs `{{project_name}}` substituted at generate time — a
  minimal templating approach (basic string replace) is enough here; save a
  real template engine (tera/handlebars) for when `generate` subcommands
  need field interpolation.

---

## Step 5 — Implement `commands::init`

`src/commands/init.rs`:

```rust
pub fn run(
    name: &str,
    package_manager: &PackageManager,
    skip_install: bool,
    skip_git: bool,
    fs: &dyn FileSystem,
) -> anyhow::Result<()> {
    let root = PathBuf::from(name);
    for (path, contents) in nestjs_starter::files(name) {
        fs.write_file(&root.join(path), &contents)?;
    }

    if !skip_git {
        run_command("git", &["init"], &root)?;
    }
    if !skip_install {
        run_command(package_manager.install_cmd(), &["install"], &root)?;
    }

    Ok(())
}
```

`run_command` is a thin wrapper around `std::process::Command` — this is
the second DI seam worth flagging: if you want `init` unit-testable end to
end (not just the file-writing part), wrap process execution behind a
trait too (`trait CommandRunner`) with a fake for tests. Otherwise, cover
it with one real integration test instead (see Step 7).

Wire into `commands/mod.rs`:

```rust
Some(Commands::Init { name, package_manager, skip_install, skip_git }) => {
    commands::init::run(name, package_manager, *skip_install, *skip_git, &RealFileSystem)?;
}
```

**Checkpoint:** `cargo run -- init my-api` produces a real, runnable NestJS
project (`cd my-api && npm run start:dev` works).

---

## Step 6 — Tests

- Unit test: template file list renders with `{{project_name}}` correctly
  substituted, using `InMemoryFileSystem` (no real I/O, no `npm`/`git`).
- Integration test (`tests/init.rs`), gated behind a feature or `#[ignore]`
  if it shells out to real `npm`/`git`: run `init` into a `tempdir`, assert
  key files exist, optionally assert `npm install` succeeds.

---

## Suggested implementation order

1. Step 0 — remove demo commands, confirm clean build.
2. Step 3 (`FileSystem` trait) — small and self-contained.
3. Step 2 (args) — get `init` parsing end to end (`todo!()` body).
4. Step 4 (embed starter template) — static content, no logic yet.
5. Step 5 — wire template writing + git/npm invocation.
6. Step 6 — tests alongside, not deferred.

---

## Roadmap (not now): `generate` subcommands

Once `init` works, the same `FileSystem` + embedded-template pattern
extends naturally to `nest generate`-equivalents, one subcommand per link
already collected in `nestjs-urls.txt`:

| Command | Docs |
|---|---|
| `generate controller <name>` | [controllers](https://docs.nestjs.com/controllers) |
| `generate provider <name>` | [providers](https://docs.nestjs.com/providers) |
| `generate module <name>` | [modules](https://docs.nestjs.com/modules) |
| `generate middleware <name>` | [middleware](https://docs.nestjs.com/middleware) |
| `generate filter <name>` | [exception filters](https://docs.nestjs.com/exception-filters) |
| `generate pipe <name>` | [pipes](https://docs.nestjs.com/pipes) |
| `generate guard <name>` | [guards](https://docs.nestjs.com/guards) |
| `generate decorator <name>` | [custom decorators](https://docs.nestjs.com/custom-decorators) |

Each of these is a smaller version of the same shape as `init`: a template
(or a few, e.g. controller + its spec file) plus `{{name}}` substitution,
written through the same `FileSystem` trait — no new architecture needed,
just more templates and match arms.
