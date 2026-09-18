//! Qualify published IcyDB admission through Canic's single bootstrap policy.
//!
//! Native current-format substrate tests; these do not authorize cross-release
//! migration or replace the composed canister's PocketIC lifecycle proof.

use canic::memory::{
    CanicMemoryManagerPolicy, admission::MemoryBootstrapAdmission, registry::MemoryRegistryError,
};
use ic_memory::{
    BootstrapAdmission, MemoryManagerAuthorityRecord, MemoryManagerConfig, MemoryManagerIdRange,
    MemoryManagerRangeMode, MemoryRequest, MemoryRuntime, PolicyIdentity, RuntimeBootstrapError,
    RuntimeOpenError, SchemaMetadata, SealedDeclarationSnapshot, StableKey,
    StaticMemoryRangeDeclaration,
    ic_stable_structures::{Memory, VectorMemory},
};
use icydb::db::{MemoryBootstrapAdmissionError, prepare_memory_bootstrap};
use std::sync::atomic::{AtomicUsize, Ordering};

fn prepare(admission: &mut BootstrapAdmission<'_>) -> Result<(), MemoryRegistryError> {
    prepare_memory_bootstrap(admission).map_err(|source| MemoryRegistryError::Admission {
        source: Box::new(source),
    })
}

fn policy() -> CanicMemoryManagerPolicy {
    CanicMemoryManagerPolicy::new().with_admission(MemoryBootstrapAdmission::new(
        PolicyIdentity::new("icydb.logical-memory-admission", 1).unwrap(),
        prepare,
    ))
}

fn snapshot(databases: &[(&str, u8, &[&str])]) -> SealedDeclarationSnapshot {
    let mut requests = Vec::new();
    let mut ranges = Vec::new();
    for &(namespace, start, stores) in databases {
        let owner = format!("icydb.{namespace}");
        ranges.push(
            StaticMemoryRangeDeclaration::new(
                MemoryManagerAuthorityRecord::new(
                    MemoryManagerIdRange::new(start, start + 15).unwrap(),
                    &owner,
                    MemoryManagerRangeMode::Allowed,
                    None,
                )
                .unwrap(),
            )
            .unwrap(),
        );
        let mut roles = vec![
            "commit.control.v1".to_owned(),
            "startup.control.v1".to_owned(),
            "integrity.progress.v1".to_owned(),
        ];
        for store in stores {
            for role in ["data", "index", "schema", "journal"] {
                roles.push(format!("store.{store}.{role}.v1"));
            }
        }
        requests.extend(roles.into_iter().map(|role| {
            MemoryRequest::new(
                &owner,
                &format!("{owner}.{role}"),
                SchemaMetadata::default(),
            )
            .unwrap()
        }));
    }
    SealedDeclarationSnapshot::new(&[], &ranges, &requests).unwrap()
}

fn runtime(backing: &VectorMemory) -> MemoryRuntime<VectorMemory> {
    MemoryRuntime::new_with_config(backing.clone(), MemoryManagerConfig::new(1).unwrap()).unwrap()
}

fn id(runtime: &MemoryRuntime<VectorMemory>, key: &str) -> u8 {
    runtime
        .committed_allocations()
        .unwrap()
        .slot_for(&StableKey::parse(key).unwrap())
        .unwrap()
        .memory_manager_id()
        .unwrap()
}

#[test]
fn composed_admission_is_once_per_cold_bootstrap_and_identity_bound() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    fn counted(admission: &mut BootstrapAdmission<'_>) -> Result<(), MemoryRegistryError> {
        CALLS.fetch_add(1, Ordering::SeqCst);
        prepare(admission)
    }
    let identity = PolicyIdentity::new("test.icydb-admission", 1).unwrap();
    let policy = CanicMemoryManagerPolicy::new()
        .with_admission(MemoryBootstrapAdmission::new(identity.clone(), counted));
    let declarations = snapshot(&[("first", 100, &["rows"]), ("second", 120, &["rows"])]);
    let mut runtime = runtime(&VectorMemory::default());
    runtime.bootstrap(&declarations, &policy).unwrap();
    runtime.bootstrap(&declarations, &policy).unwrap();
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
    assert!((100..=115).contains(&id(&runtime, "icydb.first.store.rows.data.v1")));
    assert!((120..=135).contains(&id(&runtime, "icydb.second.store.rows.data.v1")));
    let changed = CanicMemoryManagerPolicy::new().with_admission(MemoryBootstrapAdmission::new(
        identity.with_configuration_digest([7; 32]),
        counted,
    ));
    assert!(matches!(
        runtime.bootstrap(&declarations, &changed),
        Err(RuntimeBootstrapError::PolicyIdentityMismatch { .. })
    ));
    assert!(matches!(
        runtime.bootstrap(&declarations, &CanicMemoryManagerPolicy::new()),
        Err(RuntimeBootstrapError::PolicyIdentityMismatch { .. })
    ));
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
}

#[test]
fn removed_namespace_rejects_with_typed_error_without_committing() {
    let backing = VectorMemory::default();
    let declarations = snapshot(&[("first", 100, &["rows"]), ("second", 120, &["rows"])]);
    runtime(&backing)
        .bootstrap(&declarations, &policy())
        .unwrap();
    let before = backing.borrow().clone();
    let mut reopened = runtime(&backing);
    let error = reopened
        .bootstrap(&snapshot(&[("first", 100, &["rows"])]), &policy())
        .unwrap_err();
    let RuntimeBootstrapError::AdmissionPolicy(MemoryRegistryError::Admission { source }) = error
    else {
        panic!("expected typed admission rejection: {error:?}");
    };
    assert_eq!(
        source.downcast_ref::<MemoryBootstrapAdmissionError>(),
        Some(&MemoryBootstrapAdmissionError::NamespaceRemoved(
            "second".to_owned()
        ))
    );
    assert_eq!(*backing.borrow(), before);
    reopened.bootstrap(&declarations, &policy()).unwrap();
}

#[test]
fn omitted_store_selects_only_original_journal_and_preserves_bytes() {
    let backing = VectorMemory::default();
    let journal_key = "icydb.first.store.old.journal.v1";
    let original_id = {
        let mut initial = runtime(&backing);
        initial
            .bootstrap(&snapshot(&[("first", 100, &["old"])]), &policy())
            .unwrap();
        let journal = initial.open_memory_by_key(journal_key).unwrap();
        assert_eq!(journal.grow(1), 0);
        journal.write(0, b"pending journal");
        id(&initial, journal_key)
    };
    let mut reopened = runtime(&backing);
    reopened
        .bootstrap(&snapshot(&[("first", 100, &["new"])]), &policy())
        .unwrap();
    assert_eq!(id(&reopened, journal_key), original_id);
    let journal = reopened.open_memory_by_key(journal_key).unwrap();
    let mut bytes = [0; 15];
    journal.read(0, &mut bytes);
    assert_eq!(&bytes, b"pending journal");
    for role in ["data", "index", "schema"] {
        assert!(matches!(
            reopened.open_memory_by_key(&format!("icydb.first.store.old.{role}.v1")),
            Err(RuntimeOpenError::StableKeyNotCommitted(_))
        ));
    }
    assert!(
        reopened
            .open_memory_by_key("icydb.first.store.new.data.v1")
            .is_ok()
    );
}

#[test]
fn admission_cannot_grant_canic_reserved_slots() {
    let backing = VectorMemory::default();
    let declarations = snapshot(&[("first", 100, &["rows"])]);
    runtime(&backing)
        .bootstrap(&declarations, &policy())
        .unwrap();
    let before = backing.borrow().clone();
    let mut reopened = runtime(&backing);
    assert!(matches!(
        reopened.bootstrap(
            &snapshot(&[("first", 100, &["rows"]), ("second", 30, &["rows"])]),
            &policy(),
        ),
        Err(RuntimeBootstrapError::Validation(
            ic_memory::AllocationValidationError::Policy(ic_memory::RuntimePolicyError::Custom(
                MemoryRegistryError::RangeAuthorityViolation { .. }
            ))
        ))
    ));
    assert!(!reopened.is_bootstrapped());
    assert_eq!(*backing.borrow(), before);
    reopened.bootstrap(&declarations, &policy()).unwrap();
}

#[test]
fn omitted_journal_requires_its_original_slot_to_remain_granted() {
    let backing = VectorMemory::default();
    let declarations = snapshot(&[("first", 100, &["old"])]);
    runtime(&backing)
        .bootstrap(&declarations, &policy())
        .unwrap();
    let before = backing.borrow().clone();
    let mut reopened = runtime(&backing);
    let error = reopened
        .bootstrap(&snapshot(&[("first", 120, &[])]), &policy())
        .unwrap_err();
    // ic-memory retains a failed historical selection even if the consumer
    // swallows it, so its typed grant rejection takes precedence here.
    assert!(matches!(
        error,
        RuntimeBootstrapError::Admission(ic_memory::BootstrapAdmissionError::Range {
            stable_key, ..
        }) if stable_key.as_str() == "icydb.first.store.old.journal.v1"
    ));
    assert!(!reopened.is_bootstrapped());
    assert_eq!(*backing.borrow(), before);
    reopened.bootstrap(&declarations, &policy()).unwrap();
}
