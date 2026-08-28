use super::*;
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
