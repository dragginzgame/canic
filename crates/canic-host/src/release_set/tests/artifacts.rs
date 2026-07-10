use super::*;

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
