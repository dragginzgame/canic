use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

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
