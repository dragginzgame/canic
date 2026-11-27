use crate::artifact::{MAX_ARTIFACT_BYTES, inspect};
use std::{fs, os::unix::fs::symlink, path::Path};

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

#[test]
fn endpoints_only_list_callable_function_exports() {
    // One empty function and one memory. Several export names can reference either.
    let mut wasm = b"\0asm\x01\0\0\0\x01\x04\x01\x60\0\0\x03\x02\x01\0\x05\x03\x01\0\x01".to_vec();
    let mut exports = vec![5];
    for (name, kind) in [
        ("canister_update z", 0),
        ("canister_composite_query m", 0),
        ("canister_query a", 0),
        ("canister_query memory", 2),
        ("canister_init", 0),
    ] {
        exports.push(u8::try_from(name.len()).unwrap());
        exports.extend_from_slice(name.as_bytes());
        exports.extend([kind, 0]);
    }
    wasm.extend([7, u8::try_from(exports.len()).unwrap()]);
    wasm.extend(exports);
    wasm.extend([10, 4, 1, 2, 0, 11]);
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), &wasm).unwrap();
    let methods = crate::endpoints(file.path()).unwrap();
    assert_eq!(
        super::compiled_metadata(&wasm).unwrap().endpoint_count,
        methods.len()
    );
    let actual: Vec<_> = methods.iter().map(|e| (e.mode, e.name.as_str())).collect();
    assert_eq!(
        actual,
        [("query", "a"), ("composite_query", "m"), ("update", "z")]
    );
    assert!(
        methods
            .iter()
            .all(|m| m.argument.is_none() && m.reply.is_none())
    );
    let section = b"canic:signature:v1:update:z";
    let types = b"Option < u64 >\0Result < (), Error >";
    let mut annotated = wasm.clone();
    annotated.extend([
        0,
        (1 + section.len() + types.len()) as u8,
        section.len() as u8,
    ]);
    annotated.extend(section);
    annotated.extend(types);
    let methods = crate::artifact::exported_methods(&annotated).unwrap();
    assert_eq!(methods[2].argument.as_deref(), Some("Option<u64>"));
    assert_eq!(methods[2].reply.as_deref(), Some("Result<(), Error>"));
    assert!(methods[0].argument.is_none());

    wasm.truncate(wasm.len() - 7);
    std::fs::write(file.path(), wasm).unwrap();
    assert!(crate::endpoints(file.path()).is_err());
}
