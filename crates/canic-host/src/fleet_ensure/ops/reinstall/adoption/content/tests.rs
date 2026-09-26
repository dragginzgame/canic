//! Publication archive evidence remains usable after shared object removal.

use super::*;
use crate::test_support::temp_dir;
use std::fs;

fn document(bytes: &[u8]) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "reviewed_desired": {"desired": {"bootstrap": {"coordinator": "historical"}}},
        "protocol_actions": [{"kind": "fleet_protocol", "action": {
            "kind": "publish_store_chunk", "request": {
                "bytes_sha256": sha256_hex(bytes), "bytes_size": bytes.len(),
            },
        }}],
    }))
    .unwrap()
}

#[test]
fn retained_publication_survives_shared_object_removal_and_repeat() {
    let root = temp_dir("completed-content-archive");
    let paths = EnsurePaths::under(&root, "local", "fleet");
    let bytes = b"exact historical Store publication";
    let document = document(bytes);
    let digest = sha256_hex(bytes);
    fs::create_dir_all(&paths.content).unwrap();
    let object = paths.content.join(&digest);
    fs::write(&object, bytes).unwrap();
    retain(&paths, &document).unwrap();
    let archive = super::super::object_path(&paths, &digest);
    let before = fs::metadata(&archive).unwrap().modified().unwrap();
    retain(&paths, &document).unwrap();
    assert_eq!(fs::metadata(&archive).unwrap().modified().unwrap(), before);
    fs::remove_file(object).unwrap();
    verify(&paths, &document).unwrap();
    assert_eq!(fs::read(&archive).unwrap(), bytes);
    fs::write(&archive, b"tampered").unwrap();
    assert!(matches!(
        verify(&paths, &document),
        Err(EnsureStateError::ActivationResetAdoptionConflict)
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn content_reference_substitution_rejects_before_archiving() {
    for (field, value) in [
        ("bytes_sha256", Value::from("../../outside")),
        ("bytes_sha256", Value::from("00".repeat(32))),
        ("bytes_size", Value::from(0)),
        (
            "bytes_size",
            Value::from(canic_core::CANIC_WASM_CHUNK_BYTES + 1),
        ),
    ] {
        let root = temp_dir("completed-content-rejection");
        fs::create_dir_all(&root).unwrap();
        let paths = EnsurePaths::under(&root, "local", "fleet");
        let mut document: Value = serde_json::from_slice(&document(b"source")).unwrap();
        document["protocol_actions"][0]["action"]["request"][field] = value;
        assert!(retain(&paths, &serde_json::to_vec(&document).unwrap()).is_err());
        assert!(
            !paths
                .plan
                .with_file_name("activation-reset-evidence")
                .exists()
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
#[ignore = "copies explicitly supplied read-only evidence into private test scratch"]
fn inspect_supplied_completed_publications_without_external_mutation() {
    let root = std::env::var_os("CANIC_COMPLETED_SOURCE_WORKSPACE").unwrap();
    let environment = std::env::var("CANIC_COMPLETED_SOURCE_ENVIRONMENT").unwrap();
    let fleet = std::env::var("CANIC_COMPLETED_SOURCE_FLEET").unwrap();
    let source = EnsurePaths::under(std::path::Path::new(&root), &environment, &fleet);
    let snapshot = crate::fleet_ensure::ops::reinstall::terminal::documents::read(
        &source,
        &environment,
        &fleet,
    )
    .unwrap();
    let mut documents = vec![source.plan.clone()];
    documents.extend(snapshot.bindings.phase_document_sha256.keys().map(|label| {
        source
            .plan
            .with_file_name("phases")
            .join(format!("{label}.json"))
    }));
    let scratch = temp_dir("completed-publication-copy");
    let local = EnsurePaths::under(&scratch, &environment, &fleet);
    fs::create_dir_all(&local.content).unwrap();
    let mut captured = Vec::new();
    let mut object_count = 0;
    for path in documents {
        let document = read_regular_bytes(&path, 32 * 1024 * 1024).unwrap();
        for (digest, size) in references(&document).unwrap() {
            let bytes = read_regular_bytes(&source.content.join(&digest), size).unwrap();
            verify_bytes(&bytes, &digest, size).unwrap();
            fs::write(local.content.join(digest), bytes).unwrap();
            object_count += 1;
        }
        retain(&local, &document).unwrap();
        captured.push(document);
    }
    assert!(object_count > 0);
    fs::remove_dir_all(&local.content).unwrap();
    for document in captured {
        verify(&local, &document).unwrap();
    }
    let after = crate::fleet_ensure::ops::reinstall::terminal::documents::read(
        &source,
        &environment,
        &fleet,
    )
    .unwrap();
    assert_eq!(after.bindings, snapshot.bindings);
    fs::remove_dir_all(scratch).unwrap();
}

#[test]
fn duplicate_references_require_identical_sizes_and_inline_is_not_a_fallback() {
    let mut document: Value = serde_json::from_slice(&document(b"source")).unwrap();
    let mut second = document["protocol_actions"][0].clone();
    second["action"]["request"]["bytes_size"] = 1.into();
    document["protocol_actions"]
        .as_array_mut()
        .unwrap()
        .push(second);
    assert!(matches!(
        references(&serde_json::to_vec(&document).unwrap()),
        Err(EnsureStateError::ActivationResetAdoptionConflict)
    ));
    document["protocol_actions"].as_array_mut().unwrap().pop();
    document["protocol_actions"][0]["action"]["request"]["bytes"] = serde_json::json!([1, 2, 3]);
    assert!(matches!(
        references(&serde_json::to_vec(&document).unwrap()),
        Err(EnsureStateError::ActivationResetAdoptionConflict)
    ));
}
