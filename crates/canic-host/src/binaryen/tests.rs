use super::*;

use std::{fs, path::PathBuf, time::SystemTime};

#[test]
fn repository_binaryen_authority_matches_every_supported_projection() {
    let pins = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tool-versions.env"
    ));
    assert_eq!(
        BINARYEN_VERSION,
        repository_pin(pins, "CANIC_BINARYEN_VERSION")
    );

    let expected_projections = [
        ("macos", "aarch64", "arm64-macos", "DARWIN_ARM64"),
        ("macos", "x86_64", "x86_64-macos", "DARWIN_X64"),
        ("linux", "x86_64", "x86_64-linux", "LINUX_X64"),
    ];

    assert_eq!(
        SUPPORTED_BINARYEN_AUTHORITIES.len(),
        expected_projections.len()
    );

    for (os, arch, archive_platform, pin_suffix) in expected_projections {
        let authority = binaryen_authority_for(os, arch).expect("supported Binaryen platform");

        assert_eq!(authority.archive_platform(), archive_platform);
        assert_eq!(
            authority.archive_sha256(),
            repository_pin(pins, &format!("CANIC_BINARYEN_SHA256_{pin_suffix}"))
        );
        assert_eq!(
            authority.executable_sha256(),
            repository_pin(
                pins,
                &format!("CANIC_BINARYEN_WASM_OPT_SHA256_{pin_suffix}")
            )
        );
    }
}

fn repository_pin<'a>(pins: &'a str, variable: &str) -> &'a str {
    let prefix = format!("export {variable}=");
    let mut values = pins.lines().filter_map(|line| line.strip_prefix(&prefix));
    let value = values.next().expect("repository Binaryen pin");

    assert!(
        values.next().is_none(),
        "duplicate repository pin {variable}"
    );
    value
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
