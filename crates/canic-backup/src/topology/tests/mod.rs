use super::*;

const ROOT: Principal = Principal::from_slice(&[]);

// Build a deterministic non-root principal for topology hash tests.
fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

// Ensure record order does not change the canonical hash.
#[test]
fn topology_hash_is_order_independent() {
    let records = vec![record(p(2), Some(ROOT), "app"), record(ROOT, None, "root")];
    let reversed = vec![record(ROOT, None, "root"), record(p(2), Some(ROOT), "app")];

    let first = TopologyHasher::hash(&records);
    let second = TopologyHasher::hash(&reversed);

    assert_eq!(first.hash, second.hash);
    assert_eq!(first.hash.len(), 64);
}

// Ensure parent changes affect the hash.
#[test]
fn topology_hash_changes_when_parent_changes() {
    let original = vec![record(p(2), Some(ROOT), "app")];
    let changed = vec![record(p(2), Some(p(3)), "app")];

    let first = TopologyHasher::hash(&original);
    let second = TopologyHasher::hash(&changed);

    assert_ne!(first.hash, second.hash);
}

// Ensure canonical input uses explicit nulls for missing optional fields.
#[test]
fn canonical_input_uses_explicit_null_markers() {
    let input = TopologyHasher::canonical_input(&[record(ROOT, None, "root")]);

    assert!(input.contains("parent_pid=null"));
    assert!(input.contains("module_hash=null"));
}

// Build one topology record for tests.
fn record(pid: Principal, parent_pid: Option<Principal>, role: &str) -> TopologyRecord {
    TopologyRecord {
        pid,
        parent_pid,
        role: role.to_string(),
        module_hash: None,
    }
}
