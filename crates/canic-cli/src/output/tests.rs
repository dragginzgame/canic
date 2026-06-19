use super::*;
use crate::test_support::temp_dir;
use serde_json::json;

// Ensure --out style JSON writes can create nested output directories.
#[test]
fn write_pretty_json_creates_parent_directories() {
    let root = temp_dir("canic-cli-output-parent");
    let out = root.join("reports/nested/summary.json");

    write_pretty_json::<_, Box<dyn std::error::Error>>(Some(&out), &json!({"ok": true}))
        .expect("write json");

    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(&out).expect("read json")).expect("parse json");
    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(value["ok"], true);
}

// Ensure current-directory output names do not try to create an empty parent.
#[test]
fn ensure_parent_dir_accepts_plain_relative_file_names() {
    ensure_parent_dir::<io::Error>(Path::new("summary.json")).expect("plain output path");
}
