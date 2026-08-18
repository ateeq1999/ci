//! Consistent status messages across every command, so output reads the
//! same regardless of which command produced it — a prefix per kind
//! (in-progress, success, warning, failure), nothing more.

pub fn step(msg: &str) {
    println!("→ {msg}");
}

pub fn success(msg: &str) {
    println!("✓ {msg}");
}

pub fn warn(msg: &str) {
    println!("⚠ {msg}");
}

pub fn error(msg: &str) {
    eprintln!("✗ {msg}");
}
