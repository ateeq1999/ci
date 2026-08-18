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
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize, PartialEq, Debug, Default)]
    struct Sample {
        #[serde(default)]
        name: Option<String>,
    }

    #[test]
    fn returns_cli_args_when_no_json_given() {
        let cli_args = Sample {
            name: Some("from-cli".into()),
        };
        let resolved = resolve(cli_args, None).unwrap();
        assert_eq!(resolved.name.as_deref(), Some("from-cli"));
    }

    #[test]
    fn parses_inline_json_payload() {
        let cli_args = Sample::default();
        let resolved: Sample = resolve(cli_args, Some(r#"{"name":"from-json"}"#)).unwrap();
        assert_eq!(resolved.name.as_deref(), Some("from-json"));
    }

    #[test]
    fn reads_json_payload_from_file() {
        let path = std::env::temp_dir().join("clap-tutorials-json-payload-test.json");
        std::fs::write(&path, r#"{"name":"from-file"}"#).unwrap();

        let cli_args = Sample::default();
        let resolved: Sample =
            resolve(cli_args, Some(&format!("@{}", path.display()))).unwrap();

        std::fs::remove_file(&path).unwrap();
        assert_eq!(resolved.name.as_deref(), Some("from-file"));
    }
}
