// Category C - System-level artifact test (no embedded config).

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

#[test]
fn receipt_backed_authority_and_consumer_inventory_is_explicit() {
    let root = workspace_root();

    assert_eq!(
        source_paths_containing(&root, "ReceiptBackedIntentApi::"),
        BTreeSet::from(["canisters/test/intent_authority/src/lib.rs".to_string()]),
        "the public application facade has an unreviewed in-repository consumer",
    );
    assert_eq!(
        source_paths_containing(&root, "ReceiptBackedIntentWorkflow::"),
        BTreeSet::from([
            "crates/canic-core/src/api/intent/mod.rs".to_string(),
            "crates/canic-core/src/workflow/placement/allocation.rs".to_string(),
            "crates/canic-core/src/workflow/runtime/intent.rs".to_string(),
        ]),
        "receipt workflow ownership changed without updating the 0.96 inventory",
    );
    assert_eq!(
        source_paths_containing(&root, "ReceiptBackedIntentOps::"),
        BTreeSet::from([
            "crates/canic-core/src/ops/storage/intent/tests.rs".to_string(),
            "crates/canic-core/src/workflow/placement/acknowledgement.rs".to_string(),
            "crates/canic-core/src/workflow/placement/allocation.rs".to_string(),
            "crates/canic-core/src/workflow/runtime/intent.rs".to_string(),
            "crates/canic-core/src/workflow/runtime/mod.rs".to_string(),
        ]),
        "receipt storage ops gained an unreviewed caller",
    );
    assert_eq!(
        source_paths_containing(&root, "ReceiptBackedIntentStore::"),
        BTreeSet::from([
            "crates/canic-core/src/ops/storage/intent/mod.rs".to_string(),
            "crates/canic-core/src/ops/storage/intent/tests.rs".to_string(),
            "crates/canic-core/src/storage/stable/intent.rs".to_string(),
        ]),
        "stable receipt records must remain behind the storage ops authority",
    );
}

#[test]
fn receipt_backed_stable_allocations_remain_single_owner() {
    let root = workspace_root();
    let storage = read(&root.join("crates/canic-core/src/storage/stable/intent.rs"));
    let allocations = read(&root.join("crates/canic-core/src/role_contract/allocation.rs"));

    assert_eq!(
        storage
            .matches("canic.core.receipt_backed_intent_records.v1")
            .count(),
        1,
        "receipt-backed primary stable authority must have one allocation",
    );
    assert_eq!(
        storage
            .matches("canic.core.placement_acknowledgement_index.v1")
            .count(),
        1,
        "placement acknowledgement must retain one separate derived index",
    );
    assert!(allocations.contains("pub const RECEIPT_BACKED_INTENT_RECORDS_ID: u8 = 43;"));
    assert!(allocations.contains("pub const PLACEMENT_ACKNOWLEDGEMENT_INDEX_ID: u8 = 45;"));
}

fn source_paths_containing(root: &Path, needle: &str) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for source_root in ["crates", "canisters", "fleets"] {
        collect_rust_sources(&root.join(source_root), root, &mut |path, source| {
            if path == "crates/canic-core/tests/receipt_reclamation_inventory_guard.rs" {
                return;
            }
            if source.contains(needle) {
                paths.insert(path.to_string());
            }
        });
    }
    paths
}

fn collect_rust_sources(directory: &Path, root: &Path, visit: &mut impl FnMut(&str, &str)) {
    let mut entries = fs::read_dir(directory)
        .unwrap_or_else(|err| panic!("read {}: {err}", directory.display()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|err| panic!("read entry below {}: {err}", directory.display()));
    entries.sort_by_key(std::fs::DirEntry::path);

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(&path, root, visit);
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .unwrap_or_else(|err| panic!("relativize {}: {err}", path.display()))
            .to_string_lossy()
            .replace('\\', "/");
        let source = read(&path);
        visit(&relative, &source);
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(PathBuf::from)
        .expect("workspace root")
}
