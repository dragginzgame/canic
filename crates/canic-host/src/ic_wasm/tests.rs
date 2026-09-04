use super::*;

use std::{fs, path::PathBuf, time::SystemTime};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn repository_ic_wasm_authority_matches_every_install_projection() {
    let pins = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tool-versions.env"
    ));
    assert_eq!(
        IC_WASM_VERSION,
        repository_pin(pins, "CANIC_IC_WASM_VERSION")
    );

    let expected_projections = [
        ("macos", "aarch64", "aarch64-apple-darwin", "DARWIN_ARM64"),
        ("macos", "x86_64", "x86_64-apple-darwin", "DARWIN_X64"),
        (
            "linux",
            "aarch64",
            "aarch64-unknown-linux-gnu",
            "LINUX_ARM64",
        ),
        ("linux", "x86_64", "x86_64-unknown-linux-gnu", "LINUX_X64"),
    ];

    assert_eq!(
        SUPPORTED_IC_WASM_AUTHORITIES.len(),
        expected_projections.len()
    );
    for (os, arch, archive_platform, pin_suffix) in expected_projections {
        let authority = ic_wasm_authority_for(os, arch).expect("install-capable projection");
        assert_eq!(authority.archive_platform(), archive_platform);
        assert_eq!(
            authority.archive_sha256(),
            repository_pin(pins, &format!("CANIC_IC_WASM_SHA256_{pin_suffix}"))
        );
    }
}

fn repository_pin<'a>(pins: &'a str, variable: &str) -> &'a str {
    let prefix = format!("export {variable}=");
    let mut values = pins.lines().filter_map(|line| line.strip_prefix(&prefix));
    let value = values.next().expect("repository ic-wasm pin");
    assert!(
        values.next().is_none(),
        "duplicate repository pin {variable}"
    );
    value
}

#[cfg(unix)]
#[test]
fn incompatible_path_selected_ic_wasm_is_rejected_with_its_exact_path() {
    let root = temp_root("wrong-version");
    fs::create_dir_all(&root).expect("create test root");
    let executable = root.join(IC_WASM_TOOL);
    write_executable(&executable, "#!/bin/sh\nprintf 'ic-wasm 0.9.11\\n'\n");

    let error = admit_ic_wasm_executable(&executable).expect_err("wrong version must reject");

    assert!(matches!(
        error,
        IcWasmToolError::VersionMismatch { path, actual, expected }
            if path == executable
                && actual == "ic-wasm 0.9.11"
                && expected == IC_WASM_VERSION_IDENTITY
    ));
    fs::remove_dir_all(root).expect("remove test root");
}

#[cfg(unix)]
#[test]
fn admitted_executable_records_absolute_path_and_exact_version() {
    let root = temp_root("admitted");
    fs::create_dir_all(&root).expect("create test root");
    let executable = root.join(IC_WASM_TOOL);
    write_executable(&executable, "#!/bin/sh\nprintf 'ic-wasm 0.11.1\\n'\n");

    let admitted = admit_ic_wasm_executable(&executable).expect("admit exact tool");

    assert_eq!(admitted.path(), executable);
    assert_eq!(admitted.version_identity(), IC_WASM_VERSION_IDENTITY);
    fs::remove_dir_all(root).expect("remove test root");
}

#[cfg(unix)]
#[test]
fn canonical_install_precedes_a_path_wrapper() {
    const CHILD_ENV: &str = "CANIC_TEST_IC_WASM_CANONICAL_PRECEDENCE_CHILD";

    if std::env::var_os(CHILD_ENV).is_some() {
        let admitted = resolve_required_ic_wasm().expect("resolve canonical installed ic-wasm");
        assert!(admitted.path().ends_with(".local/bin/ic-wasm"));
        assert_eq!(admitted.version_identity(), IC_WASM_VERSION_IDENTITY);
        return;
    }

    let root = temp_root("canonical-precedence");
    let canonical = root.join(".local/bin/ic-wasm");
    let path_directory = root.join("path-bin");
    let wrapper = path_directory.join(IC_WASM_TOOL);
    fs::create_dir_all(canonical.parent().expect("canonical parent"))
        .expect("create canonical bin");
    fs::create_dir_all(&path_directory).expect("create PATH bin");
    write_executable(&canonical, "#!/bin/sh\nprintf 'ic-wasm 0.11.1\\n'\n");
    write_executable(&wrapper, "#!/bin/sh\nprintf 'ic-wasm 0.9.11\\n'\n");

    let output = std::process::Command::new(std::env::current_exe().expect("current test binary"))
        .args([
            "--exact",
            "ic_wasm::tests::canonical_install_precedes_a_path_wrapper",
        ])
        .env("HOME", &root)
        .env("PATH", &path_directory)
        .env(CHILD_ENV, "1")
        .output()
        .expect("run isolated precedence test");

    assert!(
        output.status.success(),
        "isolated precedence test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_dir_all(root).expect("remove test root");
}

#[cfg(target_os = "linux")]
#[test]
fn staged_installer_closes_its_writer_before_executable_admission() {
    let root = temp_root("staged-admission");
    fs::create_dir_all(&root).expect("create test root");
    let candidate = root.join("candidate-ic-wasm");
    write_executable(&candidate, "#!/bin/sh\nprintf 'ic-wasm 0.11.1\\n'\n");
    let destination = root.join("bin/ic-wasm");

    publish_executable(&candidate, &destination)
        .expect("publish and admit closed staged executable");
    let admitted = admit_ic_wasm_executable(&destination).expect("admit published executable");

    assert_eq!(admitted.path(), destination);
    assert_eq!(admitted.version_identity(), IC_WASM_VERSION_IDENTITY);
    fs::remove_dir_all(root).expect("remove test root");
}

fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "canic-ic-wasm-{label}-{}-{nanos}",
        std::process::id()
    ))
}

#[cfg(unix)]
fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write fake executable");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .expect("make fake executable executable");
}
