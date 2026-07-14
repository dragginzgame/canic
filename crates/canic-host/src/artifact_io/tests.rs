use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

// Keep the shrink pass optional when the executable is absent.
#[test]
fn missing_ic_wasm_shrink_tool_is_nonfatal() {
    let root = unique_temp_dir("canic-missing-ic-wasm-shrink");
    fs::create_dir_all(&root).expect("create temp dir");
    let wasm_path = root.join("test.wasm");
    fs::write(&wasm_path, b"original wasm").expect("write wasm placeholder");

    let missing_tool = root.join("missing-ic-wasm");
    maybe_shrink_wasm_artifact_with_command(&missing_tool.display().to_string(), &wasm_path)
        .expect("missing ic-wasm should not fail artifact shrinking");

    assert_eq!(
        fs::read(&wasm_path).expect("read original wasm"),
        b"original wasm"
    );
    fs::remove_dir_all(root).expect("remove temp root");
}

// Replace the source artifact only after a successful shrink command.
#[cfg(unix)]
#[test]
fn successful_ic_wasm_shrink_replaces_artifact() {
    let root = unique_temp_dir("canic-successful-ic-wasm-shrink");
    fs::create_dir_all(&root).expect("create temp dir");
    let wasm_path = root.join("test.wasm");
    let command_path = root.join("ic-wasm");
    fs::write(&wasm_path, b"original wasm").expect("write wasm placeholder");
    write_executable(&command_path, "#!/bin/sh\nprintf 'shrunk wasm' > \"$3\"\n");

    maybe_shrink_wasm_artifact_with_command(&command_path.display().to_string(), &wasm_path)
        .expect("successful shrink should replace artifact");

    assert_eq!(
        fs::read(&wasm_path).expect("read shrunk wasm"),
        b"shrunk wasm"
    );
    fs::remove_dir_all(root).expect("remove temp root");
}

// Reject a present failing tool without exposing its partial output.
#[cfg(unix)]
#[test]
fn failed_ic_wasm_shrink_preserves_original_and_removes_partial_output() {
    let root = unique_temp_dir("canic-failed-ic-wasm-shrink");
    fs::create_dir_all(&root).expect("create temp dir");
    let wasm_path = root.join("test.wasm");
    let shrunk_path = wasm_path.with_extension("wasm.shrunk");
    let command_path = root.join("ic-wasm");
    fs::write(&wasm_path, b"original wasm").expect("write wasm placeholder");
    write_executable(
        &command_path,
        "#!/bin/sh\nprintf 'partial wasm' > \"$3\"\nprintf 'shrink failed' >&2\nexit 23\n",
    );

    maybe_shrink_wasm_artifact_with_command(&command_path.display().to_string(), &wasm_path)
        .expect_err("non-zero shrink command must fail");

    assert_eq!(
        fs::read(&wasm_path).expect("read original wasm"),
        b"original wasm"
    );
    assert!(!shrunk_path.exists());
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn missing_ic_wasm_metadata_tool_is_nonfatal() {
    let root = unique_temp_dir("canic-missing-ic-wasm-metadata");
    fs::create_dir_all(&root).expect("create temp dir");
    let wasm_path = root.join("test.wasm");
    let did_path = root.join("test.did");
    fs::write(&wasm_path, b"\0asm").expect("write wasm placeholder");
    fs::write(&did_path, b"service : {}").expect("write did placeholder");

    let missing_tool = root.join("missing-ic-wasm");
    embed_candid_metadata_with_command(&missing_tool.display().to_string(), &wasm_path, &did_path)
        .expect("missing ic-wasm should not fail metadata embedding");

    fs::remove_dir_all(root).expect("remove temp dir");
}

fn unique_temp_dir(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{label}-{}-{nanos}", std::process::id()))
}

#[cfg(unix)]
fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write fake executable");
    let mut permissions = fs::metadata(path)
        .expect("read fake executable metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("make fake executable runnable");
}
