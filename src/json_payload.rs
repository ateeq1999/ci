use anyhow::{Context, Result};
use serde::de::DeserializeOwned;

/// Resolves a command's arguments, letting a `--json` payload (literal JSON
/// or `@path/to/file.json`) fully replace the flag-parsed arguments when
/// present.
pub fn resolve<T: DeserializeOwned>(cli_args: T, json: Option<&str>) -> Result<T> {
    let Some(payload) = json else {
        return Ok(cli_args);
    };

    let content = match payload.strip_prefix('@') {
        Some(path) => std::fs::read_to_string(path)
            .with_context(|| format!("failed to read JSON payload file: {path}"))?,
        None => payload.to_string(),
    };

    serde_json::from_str(&content).context("failed to parse --json payload")
}

#[cfg(test)]
mod tests;
