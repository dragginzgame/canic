use super::*;

use std::{fs, path::PathBuf, time::SystemTime};

#[test]
fn repository_binaryen_authority_matches_every_supported_projection() {
    let pins = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tool-versions.env"
    ));
    for expected in [
        format!("export CANIC_BINARYEN_VERSION={BINARYEN_VERSION}"),
        "export CANIC_BINARYEN_SHA256_DARWIN_ARM64=375c3df6d2722ae8e56d577c4c27eacab43c75ceaaefec0861a5ac4b81612010".to_string(),
        "export CANIC_BINARYEN_SHA256_DARWIN_X64=d7091c41473cc431f8ed47ed3b8396e1443e662c88ef1d49c5a737d6b9cddcd7".to_string(),
        "export CANIC_BINARYEN_SHA256_LINUX_X64=7bb8a2d97214f40bf34abc31d49b34aa5deab10b25d6d13c5f72cb395cf142fb".to_string(),
        "export CANIC_BINARYEN_WASM_OPT_SHA256_DARWIN_ARM64=d1fb2d189fa4305889a99136aaf0ff21fe9551a764b665c7f34dfa3834a4717a".to_string(),
        "export CANIC_BINARYEN_WASM_OPT_SHA256_DARWIN_X64=e233a27614ac30ae192c1102ea8f1d0b072e06215ec3818d8d8dd79c0ef7b39e".to_string(),
        "export CANIC_BINARYEN_WASM_OPT_SHA256_LINUX_X64=36f78112c8d629e27f8c68be89bee47c245cbde8794e1ff56c03212c02dc8484".to_string(),
    ] {
        assert!(pins.lines().any(|line| line == expected));
    }
}

#[cfg(unix)]
#[test]
fn same_version_executable_with_wrong_digest_is_rejected_before_execution() {
    let root = temp_root("wrong-digest");
    fs::create_dir_all(&root).expect("create test root");
    let executable = root.join("wasm-opt");
    fs::write(
        &executable,
        "#!/bin/sh\nprintf 'this must not execute' > execution-marker\nprintf 'wasm-opt version 108 (version_108)\\n'\n",
    )
    .expect("write fake executable");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
        .expect("make fake executable runnable");

    let error = admit_binaryen_executable(&executable, &"0".repeat(64))
        .expect_err("wrong executable digest must reject");
    let message = error.to_string();

    assert!(matches!(
        error,
        BinaryenToolError::ExecutableHashMismatch { path, .. } if path == executable
    ));
    assert!(message.contains(executable.to_string_lossy().as_ref()));
    assert!(message.contains(BINARYEN_REPAIR_COMMAND));
    assert!(!root.join("execution-marker").exists());
    fs::remove_dir_all(root).expect("remove test root");
}

#[cfg(unix)]
#[test]
fn admitted_executable_records_exact_path_version_and_digest() {
    let root = temp_root("admitted");
    fs::create_dir_all(&root).expect("create test root");
    let executable = root.join("wasm-opt");
    fs::write(
        &executable,
        "#!/bin/sh\nprintf 'wasm-opt version 108 (version_108)\\n'\n",
    )
    .expect("write fake executable");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
        .expect("make fake executable runnable");
    let digest = sha256_file(&executable).expect("hash fake executable");

    let admitted = admit_binaryen_executable(&executable, &digest).expect("admit exact executable");

    assert_eq!(admitted.path(), executable);
    assert_eq!(admitted.version_identity(), BINARYEN_VERSION_IDENTITY);
    assert_eq!(admitted.sha256(), digest);
    fs::remove_dir_all(root).expect("remove test root");
}

fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "canic-binaryen-{label}-{}-{nanos}",
        std::process::id()
    ))
}
