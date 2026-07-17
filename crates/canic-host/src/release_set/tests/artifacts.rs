use super::*;

#[test]
fn selected_network_artifact_root_never_falls_back_to_local() {
    let temp = TempWorkspace::new();
    fs::create_dir_all(temp.path().join(".icp/local/canisters")).expect("create local artifacts");

    assert_eq!(
        resolve_artifact_root(temp.path(), "ic")
            .expect_err("selected network must not use local artifacts"),
        ArtifactRootError::Missing {
            artifact_root: temp.path().join(".icp/ic/canisters"),
        }
    );
    fs::create_dir_all(temp.path().join(".icp/ic/canisters"))
        .expect("create selected-network artifacts");
    assert_eq!(
        resolve_artifact_root(temp.path(), "ic").expect("selected root"),
        temp.path().join(".icp/ic/canisters")
    );
}

#[test]
fn read_release_artifact_accepts_gzip_wasm() {
    let temp = TempWorkspace::new();
    let path = temp.path().join("artifact.wasm.gz");
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00])
        .expect("write wasm bytes");
    fs::write(&path, encoder.finish().expect("finish encoder")).expect("write artifact");

    let artifact = read_release_artifact(&path).expect("read artifact");

    assert!(!artifact.is_empty());
}

#[test]
fn read_release_artifact_rejects_plain_wasm() {
    let temp = TempWorkspace::new();
    let path = temp.path().join("artifact.wasm");
    fs::write(&path, [0x00, 0x61, 0x73, 0x6d]).expect("write plain wasm");

    read_release_artifact(&path).expect_err("plain wasm must reject");
}

#[test]
fn read_release_artifact_rejects_non_wasm_payload() {
    let temp = TempWorkspace::new();
    let path = temp.path().join("artifact.bin.gz");
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(b"not wasm").expect("write payload");
    fs::write(&path, encoder.finish().expect("finish encoder")).expect("write artifact");

    read_release_artifact(&path).expect_err("non-wasm gzip payload must reject");
}

#[test]
fn release_artifact_path_resolves_inside_icp_root() {
    let temp = TempWorkspace::new();
    let artifact_dir = temp.path().join(".icp/local/canisters/app");
    fs::create_dir_all(&artifact_dir).expect("create artifact directory");
    let artifact_path = artifact_dir.join("app.wasm.gz");
    fs::write(&artifact_path, b"artifact").expect("write artifact");

    let resolved =
        resolve_release_artifact_path(temp.path(), ".icp/local/canisters/app/app.wasm.gz")
            .expect("contained artifact resolves");

    assert_eq!(
        resolved,
        artifact_path.canonicalize().expect("canonical artifact")
    );
}

#[test]
fn release_artifact_path_rejects_nonrelative_components() {
    let temp = TempWorkspace::new();

    for path in ["", "../outside.wasm.gz", "/tmp/outside.wasm.gz"] {
        assert!(
            resolve_release_artifact_path(temp.path(), path).is_err(),
            "nonrelative path must reject: {path}"
        );
    }
}

#[cfg(unix)]
#[test]
fn release_artifact_path_rejects_symlink_escape() {
    use std::os::unix::fs::symlink;

    let temp = TempWorkspace::new();
    let outside = TempWorkspace::new();
    let outside_artifact = outside.path().join("app.wasm.gz");
    fs::write(&outside_artifact, b"artifact").expect("write outside artifact");
    symlink(outside.path(), temp.path().join("linked-artifacts")).expect("link outside directory");

    let result = resolve_release_artifact_path(temp.path(), "linked-artifacts/app.wasm.gz");

    assert!(result.is_err(), "symlink escape must reject");
}
