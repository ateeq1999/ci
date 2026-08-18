# Plan: self-contained command modules (NestJS-module-style)

Goal: restructure so every command (`init` today, `generate controller` /
`generate module` / etc. tomorrow) is a self-contained folder owning its
own args, config, and template *wiring* — mirroring how a NestJS feature
module bundles its controller, service, and DTOs instead of scattering
them across `src/controllers/`, `src/services/`, `src/dtos/`.

Two things stay fixed, by design:

- **`/templates` at the project root** stays the single, fixed location for
  actual generated-code content (`.ts`, `package.json`, etc.). It does not
  move into `src/commands/<name>/`.
- **A `templates.rs` living inside a command folder is config, not
  content** — it's the manifest that says *which* root template files this
  command renders and *how* (substitution keys, output paths). The real
  `.ts`/`.json` bytes always live under root `/templates`.

This keeps the "one obvious place for real template assets" property people
rely on when hand-editing a starter template, while still giving each
command everything else it owns in one folder.

---

## Target layout

```
templates/                       # FIXED root — real template content only
  init/                          # renamed from nestjs-starter, 1:1 with the command
    package.json
    tsconfig.json
    tsconfig.build.json
    nest-cli.json
    src/
      main.ts
      app.module.ts
      app.controller.ts
      app.service.ts
      app.controller.spec.ts

src/
  main.rs
  args.rs                        # Cli + Commands enum only — thin, wires to command Args types
  context.rs                     # DI container: Context { fs, commands } — see below
  fs.rs                          # FileSystem trait (unchanged)
  json_payload.rs                # --json resolve() (unchanged, still generic over any command's Args)
  commands/
    mod.rs                       # dispatch: match cli.command, resolve json, call <cmd>::run(&args, &ctx)
    init/
      mod.rs                     # pub use args::Args; pub fn run(args: &Args, ctx: &Context) -> Result<()>
      args.rs                    # Args (clap::Args + Deserialize), PackageManager
      config.rs                  # command-internal constants (NestJS/Node versions, schema URLs, ...)
      templates.rs                # manifest: which templates/init/* files, output paths, substitutions
```

Every future command follows the same four-file shape
(`mod.rs`, `args.rs`, `config.rs`, `templates.rs`), so "add a command" is
always "add a folder," never "touch five shared files."

---

## The DI pattern: a `Context` built once, passed everywhere

Today `commands::run` hardcodes `&RealFileSystem` at the call site, and
`init::run` shells out to `git`/`npm` directly via `std::process::Command`.
Two problems: every command would repeat that hardcoding, and the process
calls aren't swappable in tests the way file writes already are.

Fix: introduce one more trait for process execution, bundle both traits
into a `Context`, and build it once at the top:

```rust
// src/fs.rs (unchanged)
pub trait FileSystem {
    fn write_file(&self, path: &Path, contents: &str) -> Result<()>;
}

// src/context.rs (new)
pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str], cwd: &Path) -> Result<()>;
}

pub struct RealCommandRunner;
impl CommandRunner for RealCommandRunner {
    fn run(&self, program: &str, args: &[&str], cwd: &Path) -> Result<()> {
        // current std::process::Command logic from commands/init.rs moves here
    }
}

pub struct Context {
    pub fs: Box<dyn FileSystem>,
    pub commands: Box<dyn CommandRunner>,
}

impl Context {
    pub fn real() -> Self {
        Self { fs: Box::new(RealFileSystem), commands: Box::new(RealCommandRunner) }
    }
}
```

Test double, reusable by every command's tests, not just `init`'s:

```rust
pub struct NoopCommandRunner { pub calls: RefCell<Vec<String>> }
impl CommandRunner for NoopCommandRunner {
    fn run(&self, program: &str, args: &[&str], _cwd: &Path) -> Result<()> {
        self.calls.borrow_mut().push(format!("{program} {}", args.join(" ")));
        Ok(())
    }
}
```

`commands::run` builds one `Context` and threads a reference through:

```rust
pub fn run(cli: &Cli) -> Result<()> {
    let ctx = Context::real();
    match &cli.command {
        Commands::Init(args) => {
            let args = json_payload::resolve(args.clone(), cli.json.as_deref())?;
            commands::init::run(&args, &ctx)
        }
        // future: Commands::GenerateController(args) => commands::generate_controller::run(&args, &ctx),
    }
}
```

Every command's `run(args: &Args, ctx: &Context)` now depends only on the
two traits it needs (`ctx.fs`, `ctx.commands`) — never on `std::fs` or
`std::process::Command` directly. That's the actual DI: dependencies are
passed in, not reached for.

---

## What goes in each of the four files (using `init` as the example)

**`args.rs`** — user-facing input, unchanged in spirit from today's
`InitArgs`; just relocated:

```rust
#[derive(Args, Deserialize, Clone, Debug, Default)]
pub struct Args {
    pub name: Option<String>,
    #[arg(long, value_enum, default_value = "npm")]
    #[serde(default)]
    pub package_manager: PackageManager,
    #[arg(long)] #[serde(default)] pub skip_install: bool,
    #[arg(long)] #[serde(default)] pub skip_git: bool,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager { #[default] Npm, Pnpm, Yarn }
```

**`config.rs`** — internal constants this command needs that are *not*
user-facing and *not* template content — the kind of thing that would be a
NestJS module's `constants.ts`:

```rust
pub const NEST_CLI_SCHEMA_URL: &str = "https://json.schemastore.org/nest-cli";
pub const STARTER_PACKAGE_VERSION: &str = "0.0.1";
pub const NODE_ENGINE_RANGE: &str = ">=20";
```

(Today these are baked directly into the template files; pulling the ones
that might reasonably change — versions, URLs — into `config.rs` means
updating them doesn't require touching template content.)

**`templates.rs`** — manifest only, still points at the fixed root
`templates/init/` folder via `include_str!`:

```rust
const FILES: &[(&str, &str)] = &[
    ("package.json", include_str!("../../../templates/init/package.json")),
    ("tsconfig.json", include_str!("../../../templates/init/tsconfig.json")),
    // ...
];

pub fn starter_files(project_name: &str) -> Vec<(PathBuf, String)> {
    FILES.iter()
        .map(|(path, contents)| (PathBuf::from(path), contents.replace("{{project_name}}", project_name)))
        .collect()
}
```

This is exactly today's `src/templates/nestjs.rs`, moved and renamed —
the "templates in the command dir are config, not content" rule in
practice.

**`mod.rs`** — orchestration, using `ctx` instead of hardcoded types:

```rust
mod args;
mod config;
mod templates;

pub use args::Args;

pub fn run(args: &Args, ctx: &Context) -> Result<()> {
    let name = args.name.as_deref().context("`name` is required ...")?;
    let root = PathBuf::from(name);

    for (path, contents) in templates::starter_files(name) {
        ctx.fs.write_file(&root.join(path), &contents)?;
    }
    if !args.skip_git {
        ctx.commands.run("git", &["init"], &root)?;
    }
    if !args.skip_install {
        ctx.commands.run(args.package_manager.command(), &["install"], &root)?;
    }
    println!("Created NestJS project in {}", root.display());
    Ok(())
}
```

---

## `args.rs` at the root shrinks to wiring only

```rust
// src/args.rs
use crate::commands::init;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: Option<String>,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new NestJS project
    Init(init::Args),
}
```

Note: `args` importing `commands::init::Args` while `commands/mod.rs`
imports `args::{Cli, Commands}` is a two-way reference between sibling
modules — that's fine in Rust (module resolution within one crate isn't
order-sensitive the way crate-level dependencies are); it only becomes a
smell if actual *logic* starts depending both ways, which it won't here —
`args` only reaches into `commands::<name>` for the `Args` type.

---

## Migration steps (from current code)

1. `git mv templates/nestjs-starter templates/init` — root folder rename,
   confirms the "fixed root, 1:1 with command name" convention.
2. Create `src/commands/init/` folder; move `src/args/mod.rs`'s
   `InitArgs`/`PackageManager` into `src/commands/init/args.rs` (renaming
   `InitArgs` → `Args` since it's now namespaced as `init::Args`).
3. Move `src/templates/nestjs.rs` → `src/commands/init/templates.rs`,
   fixing the `include_str!` relative paths for the new depth and the
   `templates/init/` rename. Delete `src/templates/` (now empty).
4. Add `src/commands/init/config.rs` with the constants currently
   hardcoded in the template files that are likely to change independently
   (versions, schema URL) — optional but establishes the pattern.
5. Move the `run_command` process-execution helper out of
   `src/commands/init.rs` into `src/context.rs` as `RealCommandRunner`;
   add the `CommandRunner` trait and `Context` struct there.
6. Rewrite `src/commands/init.rs` as `src/commands/init/mod.rs`, taking
   `ctx: &Context` instead of `fs: &dyn FileSystem`, and using
   `ctx.commands.run(...)` instead of calling `std::process::Command`
   directly.
7. Update `src/commands/mod.rs` to build `Context::real()` once and pass
   `&ctx` into `init::run`.
8. Update `src/args/mod.rs` (root) to import `commands::init::Args` per
   the wiring shown above.
9. Update tests: `commands::init::tests` now construct a `NoopCommandRunner`
   + `InMemoryFileSystem` inside a `Context` instead of passing
   `&InMemoryFileSystem` alone — lets tests also assert *which* shell
   commands would have run, not just which files would have been written.
10. `cargo build && cargo test` — should be a pure refactor, no behavior
    change; same CLI flags, same `--json` support, same generated files.

---

## Adding the next command later (e.g. `generate controller`)

1. `mkdir src/commands/generate_controller/` +
   `mkdir templates/generate-controller/` (root, fixed).
2. Drop the controller/spec template files under
   `templates/generate-controller/`.
3. `args.rs`: `pub struct Args { pub name: Option<String> }` (deriving
   `Args` + `Deserialize`, same as `init`).
4. `templates.rs`: manifest pointing at
   `templates/generate-controller/*.ts`.
5. `mod.rs`: `run(args: &Args, ctx: &Context)`, using `ctx.fs` only (no
   process calls needed for a single-file generator).
6. Add `Commands::GenerateController(generate_controller::Args)` to the
   root enum, one match arm in `commands::run`.

No shared file needs editing beyond that one match arm and enum variant —
the self-containment goal.
