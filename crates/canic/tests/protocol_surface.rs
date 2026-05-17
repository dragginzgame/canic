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

#[test]
fn wasm_store_exposes_standard_cycle_tracker() {
    let did_path = workspace_root().join("crates/canic-wasm-store/wasm_store.did");
    let did = read_text(&did_path);

    assert!(
        did.contains("type PageRequest = record { offset : nat64; limit : nat64 };")
            && did.contains("  canic_cycle_tracker : (PageRequest) -> ("),
        "missing `canic_cycle_tracker` method in {}",
        did_path.display()
    );
    assert!(
        did.contains("type CycleTopupEvent = record")
            && did.contains("  canic_cycle_topups : (PageRequest) -> ("),
        "missing `canic_cycle_topups` method in {}",
        did_path.display()
    );
}

#[test]
fn wasm_store_keeps_memory_diagnostics_out_of_default_surface() {
    let did_path = workspace_root().join("crates/canic-wasm-store/wasm_store.did");
    let did = read_text(&did_path);

    assert!(
        !did.contains("  canic_memory_registry :"),
        "unexpected `canic_memory_registry` method in {}",
        did_path.display()
    );
    assert!(
        !did.contains("  canic_memory_ledger :"),
        "unexpected `canic_memory_ledger` method in {}",
        did_path.display()
    );
}

#[test]
fn memory_ledger_diagnostic_bypasses_normal_dispatch() {
    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/shared.rs");
    let source = read_text(&macro_path);
    let endpoint = source
        .split("fn canic_memory_ledger()")
        .nth(1)
        .expect("memory ledger endpoint should exist");
    let prefix = source
        .split("fn canic_memory_ledger()")
        .next()
        .expect("source should have endpoint prefix");
    let preceding_attribute = prefix
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with("#["))
        .expect("memory ledger endpoint should have an attribute");

    assert!(
        preceding_attribute.contains("$crate::cdk::query"),
        "memory ledger diagnostic must use a raw query attribute in {}",
        macro_path.display()
    );
    assert!(
        !preceding_attribute.contains("canic_query"),
        "memory ledger diagnostic must not use normal Canic query dispatch in {}",
        macro_path.display()
    );
    assert!(
        endpoint.contains("$crate::cdk::api::is_controller")
            && endpoint.contains("MemoryQuery::ledger()"),
        "memory ledger diagnostic must be controller-gated and read the restricted ledger path"
    );
}
