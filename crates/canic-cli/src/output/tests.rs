use super::*;
use crate::test_support::temp_dir;

use serde::ser::{Error as _, SerializeMap};
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

// Ensure every shared file helper routes completed bytes through durable replacement.
#[test]
fn shared_file_helpers_publish_complete_bytes() {
    let root = temp_dir("canic-cli-output-shared-helpers");
    let json_path = root.join("json/report.json");
    let text_path = root.join("text/report.txt");

    write_pretty_json_file::<_, Box<dyn std::error::Error>>(&json_path, &json!({"ok": true}))
        .expect("write json file");
    write_text::<io::Error>(Some(&text_path), "complete text").expect("write text file");

    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(&json_path).expect("read json file"))
            .expect("parse json file")["ok"],
        true
    );
    assert_eq!(
        fs::read_to_string(&text_path).expect("read text file"),
        "complete text"
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure partial serializer output never reaches the destination or creates its parent.
#[test]
fn serialization_failure_does_not_touch_the_destination() {
    let root = temp_dir("canic-cli-output-serialization-failure");
    fs::create_dir_all(&root).expect("create temp root");
    let existing = root.join("existing.json");
    let missing_parent = root.join("missing");
    let missing = missing_parent.join("report.json");
    fs::write(&existing, b"old complete bytes").expect("write existing output");

    write_pretty_json::<_, Box<dyn std::error::Error>>(Some(&existing), &PartialThenFailure)
        .expect_err("serialization failure must reject existing output");
    write_pretty_json::<_, Box<dyn std::error::Error>>(Some(&missing), &PartialThenFailure)
        .expect_err("serialization failure must reject missing output");

    assert_eq!(
        fs::read(&existing).expect("read existing output"),
        b"old complete bytes"
    );
    assert!(!missing_parent.exists());

    fs::remove_dir_all(root).expect("remove temp root");
}

struct PartialThenFailure;

impl Serialize for PartialThenFailure {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("partial", &true)?;
        Err(S::Error::custom("injected serialization failure"))
    }
}
