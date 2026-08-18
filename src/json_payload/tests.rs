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
    let resolved: Sample = resolve(cli_args, Some(&format!("@{}", path.display()))).unwrap();

    std::fs::remove_file(&path).unwrap();
    assert_eq!(resolved.name.as_deref(), Some("from-file"));
}
