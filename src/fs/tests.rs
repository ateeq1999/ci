use super::*;

#[test]
fn in_memory_file_system_records_writes() {
    let fs = InMemoryFileSystem::default();
    fs.write_file(Path::new("project/package.json"), "{}")
        .unwrap();

    assert_eq!(
        fs.written.borrow().get(Path::new("project/package.json")),
        Some(&"{}".to_string())
    );
}
