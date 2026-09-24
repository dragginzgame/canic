//! Integration coverage for read-only build-lock inspection through the CLI boundary.

#[cfg(target_os = "linux")]
mod recovery;

use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn inspection_emits_structured_unknowns_without_creating_or_changing_lock_files() {
    let root = std::env::temp_dir().join(format!(
        "canic-lock-inspection-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let lock = root.join("lock");
    let inspect = || {
        Command::new(env!("CARGO_BIN_EXE_canic"))
            .args(["diagnostic", "build-lock", "--lock"])
            .arg(&lock)
            .arg("--json")
            .output()
            .unwrap()
    };
    let missing = inspect();
    assert!(!missing.status.success());
    assert!(!root.exists());
    fs::create_dir(&root).unwrap();
    fs::write(&lock, b"malformed metadata").unwrap();
    let result = inspect();
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["lock_path"], lock.to_str().unwrap());
    assert!(report["recorded_owner"].is_null());
    assert_eq!(report["owner_visibility"], "unavailable");
    assert_eq!(fs::read(&lock).unwrap(), b"malformed metadata");
    fs::remove_dir_all(root).unwrap();
}
