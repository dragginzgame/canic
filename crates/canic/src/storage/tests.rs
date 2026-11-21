use super::*;
use ic_storage::VectorMemory;
use std::panic::{AssertUnwindSafe, catch_unwind};

const KEY: &str = "example.worker.record.v1";
const KEYS: &[&str] = &[KEY, "example.worker.index.v1"];

#[test]
fn lower_level_directory_reopens_with_its_original_ids_and_cells() {
    let backing = VectorMemory::default();
    let config = StorageConfig::default();
    let mut old = MemoryRuntime::new(backing.clone()).unwrap();
    old.bootstrap(
        &SealedDeclarationSnapshot::new(
            KEYS.iter()
                .map(|key| MemoryRequest::new("worker", key).unwrap())
                .collect(),
        )
        .unwrap(),
        &AllocationGrants::new(vec![
            MemoryAuthority::new("worker", "example.worker.").unwrap(),
        ])
        .unwrap(),
        &GenericAllocationPolicy,
    )
    .unwrap();
    let keys = KEYS
        .iter()
        .map(|key| StableKey::parse(key).unwrap())
        .collect::<Vec<_>>();
    let ids = keys
        .iter()
        .map(|key| old.allocation_id(key).unwrap())
        .collect::<Vec<_>>();
    drop(cell(&old, KEY, 32, Some(vec![7, 9])));
    drop(old);
    let open = |keys: &[&str], config| {
        bootstrap_with_prefix(
            backing.clone(),
            false,
            "worker",
            "example.worker.",
            keys,
            config,
        )
    };
    for (names, config) in [
        (&[KEY][..], config),
        (&[KEY, "example.worker.missing.v1"][..], config),
        (
            KEYS,
            StorageConfig {
                allocations: 1,
                ..config
            },
        ),
    ] {
        assert!(catch_unwind(AssertUnwindSafe(|| open(names, config))).is_err());
    }
    let retained = open(KEYS, config);
    for (key, id) in keys.iter().zip(ids) {
        assert_eq!(retained.allocation_id(key), Some(id));
    }
    assert_eq!(cell(&retained, KEY, 32, None).get(), &[7, 9]);
}

#[test]
fn fresh_explicit_grant_admits_only_its_prefix() {
    let fresh = |keys: &[&str]| {
        bootstrap_with_prefix(
            VectorMemory::default(),
            true,
            "worker",
            "example.worker.",
            keys,
            StorageConfig::default(),
        )
    };
    assert!(catch_unwind(|| fresh(&["worker.record.v1"])).is_err());
    assert!(
        fresh(KEYS)
            .allocation_id(&StableKey::parse(KEY).unwrap())
            .is_some()
    );
}

#[test]
fn composed_bootstrap_reopens_both_owners_and_refuses_changed_membership_and_grants() {
    let memory = VectorMemory::default();
    let app = ("directory", "directory.", &["directory.rows.v1"][..]);
    let auth = (
        "auth",
        "auth.receiver.",
        &["auth.receiver.engine.v1", "auth.receiver.host.v1"][..],
    );
    let open = |fresh, groups: &[(&str, &str, &[&str])]| {
        bootstrap_groups(memory.clone(), fresh, groups, StorageConfig::default())
    };
    drop(open(true, &[app, auth]));
    assert!(catch_unwind(AssertUnwindSafe(|| open(false, &[app]))).is_err());
    let swapped = ("directory", "directory.", auth.2);
    assert!(catch_unwind(AssertUnwindSafe(|| open(false, &[app, swapped]))).is_err());
    drop(open(false, &[app, auth]));
}
