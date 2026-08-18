use super::*;

#[test]
fn noop_command_runner_records_calls() {
    let runner = NoopCommandRunner::default();
    let calls = runner.calls.clone();

    runner.run("git", &["init"], Path::new("my-api")).unwrap();
    runner
        .run("npm", &["install"], Path::new("my-api"))
        .unwrap();

    assert_eq!(calls.borrow().as_slice(), ["git init", "npm install"]);
}
