use std::fs;
use std::path::{Path, PathBuf};

// Returns the repository root so wire-surface fixtures can be read from disk.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

// Reads a checked-in protocol artifact so the test can pin the public surface.
fn read_text(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

// Keeps the checked-in DID free of the removed cycles-accept compatibility method.
#[test]
fn removed_cycles_accept_surface_stays_absent() {
    let did_path = workspace_root().join("crates/canic-wasm-store/wasm_store.did");
    let did = read_text(&did_path);

    assert!(
        !did.contains("  ic_cycles_accept : (nat) -> (nat);"),
        "unexpected `ic_cycles_accept` method in {}",
        did_path.display()
    );
    assert!(
        !did.contains("  msg_cycles_accept : (nat) -> (nat);"),
        "unexpected `msg_cycles_accept` method in {}",
        did_path.display()
    );
    assert!(
        !did.contains("  canic_ic_cycles_accept : (nat) -> (nat);"),
        "unexpected `canic_ic_cycles_accept` method in {}",
        did_path.display()
    );
}
