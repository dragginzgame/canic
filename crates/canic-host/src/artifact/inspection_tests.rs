use super::{MAX_ARTIFACT_BYTES, inspect};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

#[test]
fn exact_wasm_identity_follows_file_and_parent_aliases() {
    let root = crate::tests::directory();
    fs::create_dir(root.join("real")).unwrap();
    let file = root.join("real/module.wasm");
    fs::write(&file, b"\0asm\x01\0\0\0").unwrap();
    symlink("real", root.join("parent-alias")).unwrap();
    symlink("real/module.wasm", root.join("file-alias")).unwrap();
    fs::hard_link(&file, root.join("hard-link")).unwrap();
    for name in [
        "real/module.wasm",
        "parent-alias/module.wasm",
        "file-alias",
        "hard-link",
    ] {
        let value = inspect(&root.join(name)).unwrap();
        assert_eq!(value.bytes, 8);
        assert_eq!(
            value.sha256,
            "93a44bbb96c751218e4c00d479e4c14358122a389acca16205b1e4d0dc5f9476"
        );
    }
    assert_eq!(
        fs::read_link(root.join("file-alias")).unwrap(),
        Path::new("real/module.wasm")
    );
    assert_eq!(fs::read(&file).unwrap(), b"\0asm\x01\0\0\0");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn inspection_refuses_non_wasm_oversize_and_nonregular_inputs() {
    let root = crate::tests::directory();
    let file = root.join("module.wasm");
    fs::write(&file, b"not wasm").unwrap();
    assert!(inspect(&file).is_err());
    fs::write(&file, b"\0asm\x01\0\0\0").unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(&file)
        .unwrap()
        .set_len(MAX_ARTIFACT_BYTES as u64 + 1)
        .unwrap();
    assert!(inspect(&file).is_err());
    assert!(inspect(&root).is_err());
    let fifo = root.join("fifo");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    assert!(inspect(&fifo).is_err());
    let error = inspect(&root.join("missing")).unwrap_err();
    let mut cause: &(dyn std::error::Error + 'static) = &error;
    loop {
        if let Some(io) = cause.downcast_ref::<std::io::Error>() {
            assert_eq!(io.kind(), std::io::ErrorKind::NotFound);
            break;
        }
        cause = cause.source().expect("original missing-file cause");
    }
    symlink("missing", root.join("dangling")).unwrap();
    assert!(inspect(&root.join("dangling")).is_err());
    assert_eq!(
        fs::read_link(root.join("dangling")).unwrap(),
        Path::new("missing")
    );
    assert_eq!(
        fs::metadata(&file).unwrap().len(),
        MAX_ARTIFACT_BYTES as u64 + 1
    );
    fs::remove_dir_all(root).unwrap();
}
