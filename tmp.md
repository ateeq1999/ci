# Proposal: use `atom-engine` for the `init` templates

## Current state

[templates.rs](src/commands/init/templates.rs) bakes each starter file in via
`include_str!` and does substitution with four chained `.replace()` calls:

```rust
let rendered = contents
    .replace("{{project_name}}", project_name)
    .replace("{{package_version}}", config::STARTER_PACKAGE_VERSION)
    .replace("{{node_engine_range}}", config::NODE_ENGINE_RANGE)
    .replace("{{nest_cli_schema_url}}", config::NEST_CLI_SCHEMA_URL);
```

Works today because the templates only need flat string substitution. It
doesn't scale to anything conditional (e.g. "only include a Dockerfile if
`--docker` was passed") or looped (multiple modules/resources) without hand
rolling more string surgery.

## What atom-engine gives us

[atom-engine](https://crates.io/crates/atom-engine) (v5, by ateeq1999) is a
component-oriented template engine built on **Tera** — same `{{ var }}` /
`{% %}` syntax family as Jinja2/Django templates. Relevant pieces for us:

- `Atom::new()`, `engine.add_template(name, source)`, `engine.render(name, &ctx)` — direct replacement for the `include_str!` + `.replace()` chain, driven by a `serde_json::json!({...})` context instead of positional string args.
- Real control flow (`{% if %}`, `{% for %}`) once we need optional files or repeated blocks (e.g. multiple resource modules).
- Filters and macros if starter files grow more templated logic.
- `provide`/`inject` context, components/slots, parallel/async rendering — not needed for this use case, but there if the generator grows (e.g. sharing a `theme`/`config` object across many generated commands, not just `init`).

## Proposed integration

**Cargo.toml**

```toml
[dependencies]
atom-engine = "5"
```

**templates.rs sketch**

```rust
use atom_engine::Atom;
use serde_json::json;

const FILES: &[(&str, &str)] = &[
    (".gitignore", include_str!("../../../templates/init/.gitignore")),
    ("package.json", include_str!("../../../templates/init/package.json")),
    // ...
];

pub fn starter_files(project_name: &str) -> Result<Vec<(PathBuf, String)>> {
    let mut engine = Atom::new();
    let ctx = json!({
        "project_name": project_name,
        "package_version": config::STARTER_PACKAGE_VERSION,
        "node_engine_range": config::NODE_ENGINE_RANGE,
        "nest_cli_schema_url": config::NEST_CLI_SCHEMA_URL,
    });

    FILES
        .iter()
        .map(|(path, contents)| {
            engine.add_template(path, contents)?;
            let rendered = engine.render(path, &ctx)?;
            Ok((PathBuf::from(*path), rendered))
        })
        .collect()
}
```

Template files themselves need no syntax change — `{{project_name}}` is
already valid Tera. `.gitignore` has no placeholders, so it round-trips
untouched.

## Things to verify before merging

- `Atom::render` returns `Result`, so `starter_files`'s signature and its one
  call site in [mod.rs](src/commands/init/mod.rs) both need to go from
  infallible to `Result<...>` (small, contained change).
- Confirm none of the JSON/TS template files contain literal `{{`/`{%` that
  isn't meant to be a template variable (checked — none currently do).
- Pull in only the default feature set; `parallel`/`async`/`pool-alloc` buy
  nothing for generating ~9 small files synchronously.

## Not proposing (yet)

Component/slot features, provide/inject context, and multi-template
rendering — none of the current starter files need them. Worth revisiting if
`init` grows conditional scaffolding (e.g. `--docker`, `--testing` flags).
