use std::path::Path;

use super::*;
use crate::shared::fs::InMemoryFileSystem;

fn read(fs: &InMemoryFileSystem, path: &str) -> String {
    fs.try_read_to_string(Path::new(path)).unwrap().unwrap()
}

#[test]
fn records_a_success_entry_on_finished() {
    let fs = InMemoryFileSystem::default();
    let action = HistoryAction::new(&fs, "proj");

    action.handle(&Event::Finished {
        command: "add cache",
        message: "Caching configured".to_string(),
    });

    let contents = read(&fs, "proj/ci/history.jsonl");
    let entry: serde_json::Value = serde_json::from_str(contents.trim_end()).unwrap();
    assert_eq!(entry["command"], "add cache");
    assert_eq!(entry["status"], "success");
    assert_eq!(entry["message"], "Caching configured");
    assert!(entry["timestamp"].is_string());
}

#[test]
fn records_an_error_entry_on_error() {
    let fs = InMemoryFileSystem::default();
    let action = HistoryAction::new(&fs, "proj");

    action.handle(&Event::Error {
        command: "db migrate fresh",
        message: "no .env file found".to_string(),
    });

    let contents = read(&fs, "proj/ci/history.jsonl");
    let entry: serde_json::Value = serde_json::from_str(contents.trim_end()).unwrap();
    assert_eq!(entry["command"], "db migrate fresh");
    assert_eq!(entry["status"], "error");
    assert_eq!(entry["message"], "no .env file found");
}

#[test]
fn ignores_started_updated_and_warned_events() {
    let fs = InMemoryFileSystem::default();
    let action = HistoryAction::new(&fs, "proj");

    action.handle(&Event::Started { command: "init" });
    action.handle(&Event::Updated {
        command: "init",
        message: "Scaffolding".to_string(),
    });
    action.handle(&Event::Warned {
        command: "init",
        message: "heads up".to_string(),
    });

    assert!(
        fs.try_read_to_string(Path::new("proj/ci/history.jsonl"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn appends_rather_than_overwriting_across_multiple_commands() {
    let fs = InMemoryFileSystem::default();
    let action = HistoryAction::new(&fs, "proj");

    action.handle(&Event::Finished {
        command: "init",
        message: "Created NestJS project in my-api".to_string(),
    });
    action.handle(&Event::Finished {
        command: "add cache",
        message: "Caching configured".to_string(),
    });
    action.handle(&Event::Error {
        command: "add queue",
        message: "couldn't find `import ` in src/app.module.ts".to_string(),
    });

    let contents = read(&fs, "proj/ci/history.jsonl");
    let lines: Vec<&str> = contents.trim_end().lines().collect();
    assert_eq!(lines.len(), 3);

    let commands: Vec<String> = lines
        .iter()
        .map(|l| {
            serde_json::from_str::<serde_json::Value>(l).unwrap()["command"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(commands, ["init", "add cache", "add queue"]);
}

#[test]
fn each_line_is_a_single_self_contained_json_object() {
    let fs = InMemoryFileSystem::default();
    let action = HistoryAction::new(&fs, "proj");

    action.handle(&Event::Finished {
        command: "init",
        message: "ok".to_string(),
    });
    action.handle(&Event::Finished {
        command: "add validation",
        message: "ok".to_string(),
    });

    let contents = read(&fs, "proj/ci/history.jsonl");
    for line in contents.trim_end().lines() {
        // Each line parses on its own — this is JSONL, not a JSON array.
        serde_json::from_str::<serde_json::Value>(line).unwrap();
    }
}
