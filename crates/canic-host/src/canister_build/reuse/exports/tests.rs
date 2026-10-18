use super::*;
use crate::canister_build::reuse::tests::infrastructure_build_fixture;
use serde_json::json;
use std::{fmt::Write as _, fs};

fn messages(manifest: &Path, package: &str, input: &Path) -> Vec<u8> {
    [
        json!({"reason": "compiler-artifact", "package_id": package, "manifest_path": manifest, "target": {"kind": ["custom-build"]}}),
        json!({"reason": "build-script-executed", "package_id": package, "env": [["CANIC_CONFIG_SOURCE_PATH", input]]}),
    ].into_iter().flat_map(|message| {
        let mut bytes = serde_json::to_vec(&message).unwrap();
        bytes.push(b'\n');
        bytes
    }).collect()
}

#[test]
fn only_selected_exact_producers_admit_exports() {
    let (root, context) = infrastructure_build_fixture();
    let manifest = context
        .workspace_root
        .join("Cargo.toml")
        .canonicalize()
        .unwrap();
    let foreign = root.join("same-name/Cargo.toml");
    fs::create_dir_all(foreign.parent().unwrap()).unwrap();
    fs::copy(&manifest, &foreign).unwrap();
    let foreign = foreign.canonicalize().unwrap();
    let input = root.join("selected-config");
    let foreign_input = root.join("foreign-config");
    fs::write(&input, "selected").unwrap();
    fs::write(&foreign_input, "foreign").unwrap();
    retain(
        &context,
        &messages(&manifest, "path+file:///selected#reuse-app@0.1.0", &input),
    )
    .unwrap();
    retain(
        &context,
        &messages(
            &foreign,
            "path+file:///foreign#reuse-app@0.1.0",
            &foreign_input,
        ),
    )
    .unwrap();
    let foreign_record = record_path(&context, &foreign).unwrap();
    let producers = BTreeMap::from([(manifest, "path+file:///selected#reuse-app@0.1.0".into())]);
    for bytes in [b"\xff".as_slice(), b"not JSON"] {
        fs::write(&foreign_record, bytes).unwrap();
        let mut paths = BTreeSet::new();
        append_observed(&context, &producers, &mut paths).unwrap();
        assert_eq!(paths, BTreeSet::from([input.clone()]));
    }
    fs::remove_file(&foreign_record).unwrap();
    fs::create_dir(&foreign_record).unwrap();
    append_observed(&context, &producers, &mut BTreeSet::new()).unwrap();
    fs::remove_dir(&foreign_record).unwrap();
    #[cfg(unix)]
    {
        crate::test_support::create_fifo(&foreign_record);
        append_observed(&context, &producers, &mut BTreeSet::new()).unwrap();
        fs::remove_file(&foreign_record).unwrap();
        std::os::unix::fs::symlink(&foreign_input, &foreign_record).unwrap();
        append_observed(&context, &producers, &mut BTreeSet::new()).unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn replaced_selected_records_decline_reuse_and_current_messages_clear_exports() {
    let (root, context) = infrastructure_build_fixture();
    let manifest = context
        .workspace_root
        .join("Cargo.toml")
        .canonicalize()
        .unwrap();
    let input = root.join("selected-config");
    let package = "path+file:///selected#reuse-app@0.1.0";
    retain(&context, &messages(&manifest, package, &input)).unwrap();
    let path = record_path(&context, &manifest).unwrap();
    let bytes = fs::read(&path).unwrap();
    let producers = BTreeMap::from([(manifest.clone(), package.into())]);
    for (field, value) in [
        ("schema_version", json!(2)),
        ("package_id", json!("path+file:///foreign#reuse-app@0.1.0")),
        ("manifest_path", json!(root.join("other/Cargo.toml"))),
        ("recipe", json!("another recipe")),
    ] {
        let mut record: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        record[field] = value;
        fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
        assert!(matches!(
            append_observed(&context, &producers, &mut BTreeSet::new()),
            Err(BuildReuseError::Evidence(_))
        ));
    }
    let mut current: Vec<serde_json::Value> = messages(&manifest, package, &input)
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    current[1]["env"] = json!([]);
    let current = current
        .into_iter()
        .fold(String::new(), |mut text, message| {
            writeln!(text, "{message}").unwrap();
            text
        });
    retain(&context, current.as_bytes()).unwrap();
    let mut paths = BTreeSet::new();
    append_observed(&context, &producers, &mut paths).unwrap();
    assert!(paths.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn export_messages_require_an_unambiguous_current_producer() {
    let (root, context) = infrastructure_build_fixture();
    let manifest = context.workspace_root.join("Cargo.toml");
    let script = json!({"reason": "build-script-executed", "package_id": "unknown", "env": []});
    assert!(matches!(
        retain(&context, &serde_json::to_vec(&script).unwrap()),
        Err(BuildReuseError::Evidence(_))
    ));
    let mut ambiguous = messages(&manifest, "same identity", &root.join("input"));
    ambiguous.extend(serde_json::to_vec(&json!({"reason": "compiler-artifact", "package_id": "same identity", "manifest_path": root.join("upstream/canic/Cargo.toml"), "target": {"kind": ["custom-build"]}})).unwrap());
    assert!(matches!(
        retain(&context, &ambiguous),
        Err(BuildReuseError::Evidence(_))
    ));
    assert!(!record_path(&context, &manifest).unwrap().exists());
    fs::remove_dir_all(root).unwrap();
}
