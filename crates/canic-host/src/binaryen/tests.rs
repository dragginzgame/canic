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
        "export CANIC_BINARYEN_SHA256_DARWIN_ARM64=98aad827847af7ef990ed7098d885725c8e5b5aae75073403635617ae4e259aa".to_string(),
        "export CANIC_BINARYEN_SHA256_DARWIN_X64=40c3de90bb3766bd0282a895e139a6f50253dba49b4f5bb89e66faca162d832e".to_string(),
        "export CANIC_BINARYEN_SHA256_LINUX_X64=195ddc94f9bc89f45abdabb0b9eea86023d727ba90eac8b35b80f2544fc30572".to_string(),
        "export CANIC_BINARYEN_WASM_OPT_SHA256_DARWIN_ARM64=a9c8d09d84186e4c8efe937f3de19b887404d24a96e2638f3bd3b476e17b7218".to_string(),
        "export CANIC_BINARYEN_WASM_OPT_SHA256_DARWIN_X64=c3cbd288eef3402119d8183df1739887ff0e6430caba2e1c801406df725a2bd3".to_string(),
        "export CANIC_BINARYEN_WASM_OPT_SHA256_LINUX_X64=1014958e6f20d412f1542320b43970214b0fb1ed780595e8f7c0d8761ed53725".to_string(),
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
        "#!/bin/sh\nprintf 'this must not execute' > execution-marker\nprintf 'wasm-opt version 132 (version_132)\\n'\n",
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
        "#!/bin/sh\nprintf 'wasm-opt version 132 (version_132)\\n'\n",
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

#[cfg(target_os = "linux")]
#[test]
fn staged_installer_closes_its_writer_before_executable_admission() {
    let root = temp_root("staged-admission");
    fs::create_dir_all(&root).expect("create test root");
    let candidate = root.join("candidate-wasm-opt");
    fs::write(
        &candidate,
        "#!/bin/sh\nprintf 'wasm-opt version 132 (version_132)\\n'\n",
    )
    .expect("write fake optimizer candidate");
    fs::set_permissions(&candidate, fs::Permissions::from_mode(0o755))
        .expect("make fake optimizer candidate runnable");
    let digest = sha256_file(&candidate).expect("hash fake optimizer candidate");
    let destination = root.join("bin/wasm-opt");

    publish_executable(&candidate, &destination, &digest)
        .expect("publish and admit closed staged executable");
    let admitted =
        admit_binaryen_executable(&destination, &digest).expect("admit published executable");

    assert_eq!(admitted.path(), destination);
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
