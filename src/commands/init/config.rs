//! Internal constants baked into generated files — distinct from `args.rs`
//! (user-facing input) and `templates.rs` (which files get rendered).
//! Changing a version here doesn't require touching template content.

pub const NEST_CLI_SCHEMA_URL: &str = "https://json.schemastore.org/nest-cli";
pub const STARTER_PACKAGE_VERSION: &str = "0.0.1";
pub const NODE_ENGINE_RANGE: &str = ">=20";
