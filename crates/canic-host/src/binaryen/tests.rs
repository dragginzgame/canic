use super::*;

use std::{fs, path::PathBuf, time::SystemTime};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn repository_binaryen_authority_matches_every_supported_projection() {
    let pins = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tool-versions.env"
    ));
    assert_eq!(
        BINARYEN_VERSION,
        crate::test_support::ic_tool_pin("wasm-opt", "linux-x86_64", 1)
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
            crate::test_support::ic_tool_pin(
                "wasm-opt",
                match pin_suffix {
                    "DARWIN_ARM64" => "darwin-arm64",
                    "DARWIN_X64" => "darwin-x86_64",
                    _ => "linux-x86_64",
                },
                3
            )
        );
        assert_eq!(
            authority.executable_sha256(),
            repository_pin(
                pins,
                &format!("CANIC_BINARYEN_WASM_OPT_SHA256_{pin_suffix}")
            )
        );
        assert_eq!(
            authority.runtime_library_sha256,
            match pin_suffix {
                "LINUX_X64" => None,
                _ => Some(repository_pin(
                    pins,
                    &format!("CANIC_BINARYEN_RUNTIME_LIBRARY_SHA256_{pin_suffix}")
                )),
            }
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
        crate::test_support::tool_script("#!/bin/sh\nprintf 'this must not execute' > execution-marker\nprintf '@BINARYEN_IDENTITY@\\n'\n"),
    )
    .expect("write fake executable");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
        .expect("make fake executable runnable");

    let error = admit_binaryen_executable(&executable, &"0".repeat(64), None)
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
        crate::test_support::tool_script("#!/bin/sh\nprintf '@BINARYEN_IDENTITY@\\n'\n"),
    )
    .expect("write fake executable");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
        .expect("make fake executable runnable");
    let digest = sha256_file(&executable).expect("hash fake executable");

    let admitted =
        admit_binaryen_executable(&executable, &digest, None).expect("admit exact executable");

    assert_eq!(admitted.path(), executable);
    assert_eq!(admitted.version_identity(), BINARYEN_VERSION_IDENTITY);
    assert_eq!(admitted.sha256(), digest);
    fs::remove_dir_all(root).expect("remove test root");
}

#[cfg(unix)]
#[test]
fn canonical_install_precedes_a_path_optimizer() {
    const CHILD_ENV: &str = "CANIC_TEST_BINARYEN_CANONICAL_PRECEDENCE_CHILD";

    if std::env::var_os(CHILD_ENV).is_some() {
        let resolved = resolve_executable(OsStr::new(WASM_OPT_TOOL))
            .expect("resolve canonical installed optimizer");
        assert!(resolved.ends_with(".local/bin/wasm-opt"));
        return;
    }

    let root = temp_root("canonical-precedence");
    let canonical = root.join(".local/bin/wasm-opt");
    let path_directory = root.join("path-bin");
    let path_optimizer = path_directory.join(WASM_OPT_TOOL);
    fs::create_dir_all(canonical.parent().expect("canonical parent"))
        .expect("create canonical bin");
    fs::create_dir_all(&path_directory).expect("create PATH bin");
    write_executable(&canonical, "#!/bin/sh\nexit 0\n");
    write_executable(&path_optimizer, "#!/bin/sh\nexit 0\n");

    let output = std::process::Command::new(std::env::current_exe().expect("current test binary"))
        .args([
            "--exact",
            "binaryen::tests::canonical_install_precedes_a_path_optimizer",
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

#[cfg(unix)]
#[test]
fn explicit_relative_optimizer_precedes_the_canonical_install() {
    const CHILD_ENV: &str = "CANIC_TEST_BINARYEN_RELATIVE_CHILD";
    if std::env::var_os(CHILD_ENV).is_some() {
        let selected = resolve_executable(OsStr::new("./wasm-opt")).unwrap();
        assert_eq!(selected, std::env::current_dir().unwrap().join("wasm-opt"));
        return;
    }
    let root = temp_root("relative-selection");
    let canonical = root.join(".local/bin/wasm-opt");
    fs::create_dir_all(canonical.parent().unwrap()).unwrap();
    write_executable(&canonical, "#!/bin/sh\nexit 0\n");
    write_executable(&root.join("wasm-opt"), "#!/bin/sh\nexit 0\n");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "binaryen::tests::explicit_relative_optimizer_precedes_the_canonical_install",
        ])
        .current_dir(&root)
        .env("HOME", &root)
        .env(CHILD_ENV, "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn staged_installer_closes_its_writer_before_executable_admission() {
    let root = temp_root("staged-admission");
    fs::create_dir_all(&root).expect("create test root");
    let candidate = root.join("candidate-wasm-opt");
    fs::write(
        &candidate,
        crate::test_support::tool_script("#!/bin/sh\nprintf '@BINARYEN_IDENTITY@\\n'\n"),
    )
    .expect("write fake optimizer candidate");
    fs::set_permissions(&candidate, fs::Permissions::from_mode(0o755))
        .expect("make fake optimizer candidate runnable");
    let digest = sha256_file(&candidate).expect("hash fake optimizer candidate");
    let destination = root.join("bin/wasm-opt");

    tool_install::publish_executable(WASM_OPT_TOOL, &candidate, &destination, |path| {
        admit_binaryen_executable(path, &digest, None).map(|_| ())
    })
    .expect("publish and admit closed staged executable");
    let admitted =
        admit_binaryen_executable(&destination, &digest, None).expect("admit published executable");

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

#[cfg(unix)]
fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write fake executable");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .expect("make fake executable executable");
}

#[cfg(unix)]
#[test]
fn runtime_library_identity_is_checked_before_version_execution() {
    let root = temp_root("library-admission");
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::create_dir_all(root.join("lib")).unwrap();
    let executable = root.join("bin/wasm-opt");
    let library = root.join("lib/libbinaryen.dylib");
    let marker = root.join("executed");
    write_executable(
        &executable,
        &crate::test_support::tool_script(&format!(
            "#!/bin/sh\nprintf executed > '{}'\nprintf '@BINARYEN_IDENTITY@\\n'\n",
            marker.display()
        )),
    );
    fs::write(&library, b"qualified runtime").unwrap();
    let executable_digest = sha256_file(&executable).unwrap();
    let library_digest = sha256_file(&library).unwrap();
    fs::write(&library, b"modified runtime").unwrap();
    std::assert_matches!(
        admit_binaryen_executable(&executable, &executable_digest, Some(&library_digest)),
        Err(BinaryenToolError::RuntimeLibraryHashMismatch { .. })
    );
    assert!(!marker.exists());
    fs::remove_file(&library).unwrap();
    std::assert_matches!(
        admit_binaryen_executable(&executable, &executable_digest, Some(&library_digest)),
        Err(BinaryenToolError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound
    );
    assert!(!marker.exists());
    fs::write(&library, b"qualified runtime").unwrap();
    admit_binaryen_executable(&executable, &executable_digest, Some(&library_digest)).unwrap();
    assert!(marker.exists());
    fs::remove_dir_all(root).unwrap();
}
