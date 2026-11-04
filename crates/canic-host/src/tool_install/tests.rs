use super::*;

#[cfg(unix)]
use std::{cell::Cell, net::TcpListener, sync::mpsc, thread, time::Duration};

#[cfg(unix)]
#[test]
fn transient_download_failure_retries_without_replacing_an_installation() {
    const CHILD: &str = "CANIC_TEST_ARCHIVE_RETRY_CHILD";
    if env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(env::current_exe().unwrap())
            .args([
                "--exact",
                "tool_install::tests::transient_download_failure_retries_without_replacing_an_installation",
            ])
            .env(CHILD, "1")
            .env("NO_PROXY", "127.0.0.1")
            .env("no_proxy", "127.0.0.1")
            .env_remove("HTTPS_PROXY")
            .env_remove("https_proxy")
            .env_remove("ALL_PROXY")
            .env_remove("all_proxy")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    let temp = installation_scratch("download-retry-test").unwrap();
    let destination = temp.path().join("installed");
    fs::write(&destination, b"previous executable").unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("https://{}/tool.tar.gz", listener.local_addr().unwrap());
    thread::scope(|scope| {
        let (stop, stopped) = mpsc::channel::<()>();
        let server = scope.spawn(move || {
            let mut connections = 0;
            while matches!(stopped.try_recv(), Err(mpsc::TryRecvError::Empty)) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        connections += 1;
                        // Refuse the TLS connection before any archive can be admitted.
                        drop(stream);
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("accept local download connection: {error}"),
                }
            }
            connections
        });
        let spec = InstallSpec {
            tool: "retry-test",
            archive_name: "tool.tar.gz",
            archive_url: &url,
            archive_sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            member: "tool",
            runtime_library: None,
            format: ArchiveFormat::Gzip,
        };
        let admitted = Cell::new(false);
        let result = install(&spec, &destination, |_| {
            admitted.set(true);
            Ok::<_, InstallError>(())
        });
        drop(stop);
        assert_eq!(server.join().unwrap(), 4);
        std::assert_matches!(result, Err(InstallError::ArchiveDownload { .. }));
        assert!(!admitted.get());
        assert_eq!(fs::read(&destination).unwrap(), b"previous executable");
    });
}

#[test]
fn file_hash_preserves_pinned_identity_and_missing_file_error_kind() {
    let temp = installation_scratch("digest-test").unwrap();
    let candidate = temp.path().join("candidate");
    fs::write(&candidate, b"abc").unwrap();
    assert_eq!(
        sha256_file(&candidate).unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let missing = temp.path().join("missing");
    match sha256_file(&missing) {
        Err(InstallError::Io { path, source, .. }) => {
            assert_eq!(path, missing);
            assert_eq!(source.kind(), io::ErrorKind::NotFound);
        }
        result => panic!("expected a missing-file I/O failure, got {result:?}"),
    }
}

#[test]
fn rejected_stage_preserves_destination_and_removes_temporary_copy() {
    let temp = installation_scratch("publication-test").unwrap();
    let candidate = temp.path().join("candidate");
    let destination = temp.path().join("installed");
    fs::write(&candidate, b"new executable").unwrap();
    fs::write(&destination, b"previous executable").unwrap();
    let result = publish_executable(&candidate, &destination, |_stage| {
        Err(InstallError::ExecutableHashMismatch {
            path: destination.clone(),
            actual: "actual".into(),
            expected: "required".into(),
        })
    });
    std::assert_matches!(result, Err(InstallError::ExecutableHashMismatch { .. }));
    assert_eq!(fs::read(&destination).unwrap(), b"previous executable");
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2);
}

#[test]
fn rejected_foreign_stage_retains_original_admission_and_cleanup_causes() {
    let temp = installation_scratch("publication-custody-test").unwrap();
    let candidate = temp.path().join("candidate");
    let destination = temp.path().join("installed");
    fs::write(&candidate, b"new executable").unwrap();
    fs::write(&destination, b"previous executable").unwrap();
    let mut replacement = None;
    let result = publish_executable(&candidate, &destination, |stage| {
        fs::remove_file(stage).unwrap();
        fs::create_dir(stage).unwrap();
        replacement = Some(stage.to_path_buf());
        Err(InstallError::ExecutableHashMismatch {
            path: stage.to_path_buf(),
            actual: "replaced".into(),
            expected: "admitted".into(),
        })
    });
    let Err(InstallError::Io { source, .. }) = result else {
        panic!("expected retained publication evidence");
    };
    let cause = source
        .get_ref()
        .unwrap()
        .downcast_ref::<NamedWriteError<InstallError>>()
        .unwrap();
    std::assert_matches!(
        cause,
        NamedWriteError::Producer {
            source: InstallError::ExecutableHashMismatch { .. },
            cleanup_error: Some(_),
        }
    );
    assert!(replacement.unwrap().is_dir());
    assert_eq!(fs::read(destination).unwrap(), b"previous executable");
}

#[test]
fn archive_hash_mismatch_is_rejected_before_extraction() {
    let temp = installation_scratch("hash-test").unwrap();
    let archive = temp.path().join("archive");
    fs::write(&archive, b"wrong archive").unwrap();
    std::assert_matches!(
        verify_archive(&archive, "required"),
        Err(InstallError::ArchiveHashMismatch { .. })
    );
}

#[test]
fn publication_failure_cleans_stage_and_preserves_destination_directory() {
    let temp = installation_scratch("rename-test").unwrap();
    let candidate = temp.path().join("candidate");
    let destination = temp.path().join("installed");
    fs::write(&candidate, b"candidate").unwrap();
    fs::create_dir(&destination).unwrap();
    let result = publish_executable(&candidate, &destination, |_| Ok::<_, InstallError>(()));
    std::assert_matches!(result, Err(InstallError::Io { .. }));
    assert!(destination.is_dir());
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2);
}

#[cfg(unix)]
#[test]
fn bundle_publication_preserves_relative_library_and_previous_selection_on_rejection() {
    let temp = installation_scratch("bundle-test").unwrap();
    let candidate = temp.path().join("candidate");
    let library = temp.path().join("library");
    fs::write(&candidate, b"executable").unwrap();
    fs::write(&library, b"runtime library").unwrap();
    let destination = temp.path().join("bin/wasm-opt");
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::write(&destination, b"previous executable").unwrap();
    let admit = |path: &Path| {
        assert_eq!(fs::read(path).unwrap(), b"executable");
        assert_eq!(
            fs::read(path.parent().unwrap().join("../lib/libbinaryen.dylib")).unwrap(),
            b"runtime library"
        );
        Ok::<_, InstallError>(())
    };
    publish_bundle("wasm-opt", &candidate, &library, &destination, admit).unwrap();
    let installed = fs::canonicalize(&destination).unwrap();
    assert!(
        fs::symlink_metadata(&destination)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    admit(&installed).unwrap();
    let result = publish_bundle("wasm-opt", &candidate, &library, &destination, |stage| {
        Err(InstallError::ExecutableHashMismatch {
            path: stage.to_path_buf(),
            actual: "rejected".into(),
            expected: "required".into(),
        })
    });
    std::assert_matches!(result, Err(InstallError::ExecutableHashMismatch { .. }));
    assert_eq!(fs::canonicalize(&destination).unwrap(), installed);
    assert_eq!(
        fs::read_dir(destination.parent().unwrap()).unwrap().count(),
        2
    );
}

#[cfg(unix)]
#[test]
fn missing_runtime_library_preserves_working_installation() {
    let temp = installation_scratch("missing-library-test").unwrap();
    let candidate = temp.path().join("candidate");
    let destination = temp.path().join("installed");
    fs::write(&candidate, b"candidate").unwrap();
    fs::write(&destination, b"previous executable").unwrap();
    let result = publish_bundle(
        "wasm-opt",
        &candidate,
        &temp.path().join("missing"),
        &destination,
        |_| Ok::<_, InstallError>(()),
    );
    std::assert_matches!(result, Err(InstallError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound);
    assert_eq!(fs::read(&destination).unwrap(), b"previous executable");
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2);
}

#[cfg(unix)]
#[test]
fn bundle_executable_runs_with_its_relative_library_before_and_after_selection() {
    let temp = installation_scratch("runnable-bundle-test").unwrap();
    let candidate = temp.path().join("candidate");
    let library = temp.path().join("library");
    let destination = temp.path().join("bin/wasm-opt");
    fs::write(
        &candidate,
        b"#!/bin/sh\nread identity < \"${0%/*}/../lib/libbinaryen.dylib\"\nprintf '%s\\n' \"$identity\"\n",
    )
    .unwrap();
    fs::write(&library, b"qualified runtime\n").unwrap();
    let admit = |path: &Path| {
        let output = std::process::Command::new(path).output().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"qualified runtime\n");
        Ok::<_, InstallError>(())
    };
    publish_bundle("wasm-opt", &candidate, &library, &destination, admit).unwrap();
    admit(&fs::canonicalize(&destination).unwrap()).unwrap();
}
