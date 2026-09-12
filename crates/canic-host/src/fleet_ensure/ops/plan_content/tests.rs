use super::*;
use canic_core::dto::{
    fixture_provisioning::{
        FixtureChunkDescriptor, FixtureChunkUpload, FixtureDescriptor, FixtureSourceStatus,
    },
    root_store::{RootStoreBootstrapRequest, RootStoreFixture, RootStoreFixturePrepareRequest},
};
use std::fs;

fn paths(label: &str) -> (std::path::PathBuf, EnsurePaths) {
    let root = crate::test_support::temp_dir(&format!("plan-content-{label}"));
    let paths = EnsurePaths::under(&root, "local", "demo");
    fs::create_dir_all(&paths.content).expect("create content store");
    (root, paths)
}

#[test]
fn read_object_rejects_invalid_expected_size_before_object_access() {
    let (root, paths) = paths("declared-oversize");
    let expected = [0_u8; 32];

    let error = read_object(
        &paths,
        &expected,
        u64::try_from(canic_core::CANIC_WASM_CHUNK_BYTES).expect("chunk bound") + 1,
    )
    .expect_err("oversized expected authority must reject");

    assert!(matches!(error, EnsureStateError::StoreChunkMismatch { .. }));
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn read_object_rejects_oversized_truncated_and_hash_mismatched_content() {
    let cases: &[(&str, &[u8])] = &[
        ("oversized", b"12345"),
        ("truncated", b"123"),
        ("hash-mismatch", b"abcd"),
    ];

    for (label, retained) in cases {
        let (root, paths) = paths(label);
        let expected_bytes = b"1234";
        let expected = wasm_hash(expected_bytes);
        fs::write(object_path(&paths, &expected), retained).expect("write retained object");

        let error = read_object(&paths, &expected, expected_bytes.len() as u64)
            .expect_err("invalid retained object must reject");

        assert!(matches!(error, EnsureStateError::StoreChunkMismatch { .. }));
        fs::remove_dir_all(root).expect("remove temp root");
    }
}

#[cfg(unix)]
#[test]
fn read_object_rejects_a_linked_content_object() {
    let (root, paths) = paths("linked");
    let bytes = b"1234";
    let expected = wasm_hash(bytes);
    let outside = root.join("outside");
    fs::write(&outside, bytes).expect("write link target");
    std::os::unix::fs::symlink(&outside, object_path(&paths, &expected))
        .expect("create retained object link");

    let error = read_object(&paths, &expected, bytes.len() as u64)
        .expect_err("linked retained object must reject");

    assert!(matches!(
        error,
        EnsureStateError::StoreChunkUnavailable { .. }
    ));
    fs::remove_dir_all(root).expect("remove temp root");
}

fn fixture_projection(paths: &EnsurePaths) -> Value {
    let bytes = b"fixture rows".to_vec();
    let descriptor = FixtureDescriptor {
        schema_version: 1,
        format_hash: [1; 32],
        encoded_length: bytes.len() as u64,
        chunks: vec![FixtureChunkDescriptor {
            digest: wasm_hash(&bytes).try_into().unwrap(),
            length: u32::try_from(bytes.len()).unwrap(),
        }],
        completion_summary: [3; 32],
    };
    let content_id =
        canic_control_plane::api::fixture_content::FixtureContentApi::content_id(&descriptor)
            .unwrap();
    let source = RootStoreFixture {
        role: canic_core::ids::CanisterRole::new("app"),
        content_id,
        descriptor,
    };
    let store = candid::Principal::from_slice(&[7]);
    retain_object(paths, &wasm_hash(&bytes), &bytes).unwrap();
    let prepare = CurrentFleetProtocolAction::PrepareStoreFixture {
        maximum_attempts: 1,
        request: RootStoreFixturePrepareRequest {
            bootstrap: RootStoreBootstrapRequest {
                operation_id: [4; 32],
                manifest_payload_size_bytes: 123,
            },
            role: source.role.clone(),
        },
        store,
        source,
    };
    let publish = CurrentFleetProtocolAction::PublishStoreFixtureChunk {
        maximum_attempts: 1,
        expected: FixtureSourceStatus {
            content_id,
            next_chunk: 1,
            chunk_count: 1,
            received_bytes: bytes.len() as u64,
            complete: true,
        },
        source_bytes: bytes.len() as u64,
        request: FixtureChunkUpload {
            content_id,
            index: 0,
            bytes,
        },
    };
    serde_json::json!({"protocol_actions": [
        {"kind":"fleet_protocol", "principal":candid::Principal::from_slice(&[8]).to_text(), "action":prepare},
        {"kind":"fleet_protocol", "principal":store.to_text(), "action":publish},
    ]})
}

#[test]
fn fixture_content_round_trips_without_inline_payloads_and_rejects_substitution() {
    let (root, paths) = paths("fixture-publication");
    let original = fixture_projection(&paths);
    let mut compact = original.clone();
    remove_inline_bytes(&mut compact).unwrap();
    assert!(!contains_inline_bytes(&compact).unwrap());
    let mut hydrated = compact.clone();
    hydrate(&paths, &mut hydrated).unwrap();
    assert_eq!(hydrated, original);
    let mut cases = Vec::new();
    let mut wrong = compact.clone();
    wrong["protocol_actions"][1]["principal"] =
        Value::String(candid::Principal::anonymous().to_text());
    cases.push(wrong);
    let mut wrong = compact.clone();
    wrong["protocol_actions"][0]["action"]["request"]["role"] = Value::String("other".into());
    cases.push(wrong);
    let mut wrong = compact.clone();
    wrong["protocol_actions"].as_array_mut().unwrap().remove(0);
    cases.push(wrong);
    let mut wrong = compact.clone();
    wrong["protocol_actions"][1]["action"]["expected"]["received_bytes"] = Value::from(1);
    cases.push(wrong);
    let mut wrong = compact.clone();
    wrong["protocol_actions"][1]["action"]["source_bytes"] = Value::from(1);
    cases.push(wrong);
    let other = b"foreign rows";
    let hash = wasm_hash(other);
    retain_object(&paths, &hash, other).unwrap();
    let mut wrong = compact;
    wrong["protocol_actions"][1]["action"]["request"]["bytes_sha256"] =
        Value::String(hex_bytes(&hash));
    wrong["protocol_actions"][1]["action"]["request"]["bytes_size"] = Value::from(other.len());
    cases.push(wrong);
    for mut wrong in cases {
        std::assert_matches!(
            hydrate(&paths, &mut wrong),
            Err(EnsureStateError::StoreChunkAuthority { .. })
        );
    }
    fs::remove_dir_all(root).unwrap();
}
