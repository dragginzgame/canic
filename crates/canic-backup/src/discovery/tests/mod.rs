use super::*;
use crate::registry::RegistryEntry;

const ROOT_TEXT: &str = "aaaaa-aa";
const APP_TEXT: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const WORKER_TEXT: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

// Ensure non-recursive target resolution includes only direct children.
#[test]
fn registry_targets_include_direct_children() {
    let entries = registry_entries();
    let targets = targets_from_registry(&entries, ROOT_TEXT, false).expect("resolve targets");
    let ids = targets
        .iter()
        .map(|target| target.canister_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, vec![ROOT_TEXT, APP_TEXT]);
}

// Ensure recursive target resolution walks the full subtree.
#[test]
fn registry_targets_include_recursive_children() {
    let entries = registry_entries();
    let targets = targets_from_registry(&entries, ROOT_TEXT, true).expect("resolve targets");
    let ids = targets
        .iter()
        .map(|target| target.canister_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, vec![ROOT_TEXT, APP_TEXT, WORKER_TEXT]);
}

// Build representative subnet registry entries.
fn registry_entries() -> Vec<RegistryEntry> {
    vec![
        RegistryEntry {
            pid: ROOT_TEXT.to_string(),
            role: Some("root".to_string()),
            kind: Some("root".to_string()),
            parent_pid: None,
            module_hash: None,
        },
        RegistryEntry {
            pid: APP_TEXT.to_string(),
            role: Some("app".to_string()),
            kind: Some("singleton".to_string()),
            parent_pid: Some(ROOT_TEXT.to_string()),
            module_hash: Some("01ab".to_string()),
        },
        RegistryEntry {
            pid: WORKER_TEXT.to_string(),
            role: Some("worker".to_string()),
            kind: Some("replica".to_string()),
            parent_pid: Some(APP_TEXT.to_string()),
            module_hash: Some(HASH.to_string()),
        },
    ]
}
