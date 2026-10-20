use super::*;
use crate::canister_build::reuse::tests::infrastructure_build_fixture;
use serde_json::json;
use std::fs;

fn messages(manifest: &Path, package: &str, input: &Path) -> Vec<Message> {
    [
        json!({"reason":"compiler-artifact","package_id":package,"manifest_path":manifest,
            "target":{"name":"build-script-build","kind":["custom-build"],"crate_types":["bin"],"src_path":"/fixture/build.rs","edition":"2024","doc":false,"doctest":false,"test":false},
            "profile":{"opt_level":"0","debuginfo":0,"debug_assertions":true,"overflow_checks":true,"test":false},
            "features":[],"filenames":[],"executable":null,"fresh":false}),
        json!({"reason":"build-script-executed","package_id":package,"linked_libs":[],"linked_paths":[],"cfgs":[],"env":[["CANIC_CONFIG_SOURCE_PATH",input]],"out_dir":"/fixture/out"}),
    ].into_iter().map(|value| serde_json::from_value(value).unwrap()).collect()
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
    let mut current = messages(&manifest, package, &input);
    let Message::BuildScriptExecuted(script) = &mut current[1] else {
        panic!("fixture build script");
    };
    script.env.clear();
    retain(&context, &current).unwrap();
    let mut paths = BTreeSet::new();
    append_observed(&context, &producers, &mut paths).unwrap();
    assert!(paths.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn export_messages_require_an_unambiguous_current_producer() {
    let (root, context) = infrastructure_build_fixture();
    let manifest = context.workspace_root.join("Cargo.toml");
    let script = messages(&manifest, "unknown", &root.join("input"))
        .pop()
        .unwrap();
    assert!(matches!(
        retain(&context, &[script]),
        Err(BuildReuseError::Evidence(_))
    ));
    let mut ambiguous = messages(&manifest, "same identity", &root.join("input"));
    ambiguous.push(
        messages(
            &root.join("upstream/canic/Cargo.toml"),
            "same identity",
            &root.join("input"),
        )
        .remove(0),
    );
    assert!(matches!(
        retain(&context, &ambiguous),
        Err(BuildReuseError::Evidence(_))
    ));
    assert!(!record_path(&context, &manifest).unwrap().exists());
    fs::remove_dir_all(root).unwrap();
}
