#![cfg(test)]

use super::*;
use crate::storage::stable::env::{Env, EnvData, EnvRecord};
use canic_contracts::protocol::protected_memory_allocations::{METHOD_ID, RESPONSE_LIMITS};
use ic_storage::{Memory, MemoryRuntime, RuntimeBootstrapError, StableKey, VectorMemory};

#[test]
fn bootstrap_declares_unopened_stores_and_observation_is_effect_free() {
    MemoryRegistryOps::bootstrap_registry().unwrap();
    let before = MemoryRegistryOps::allocation_snapshot().unwrap();
    let requested = memory::requests()
        .unwrap()
        .into_iter()
        .map(|request| request.stable_key().as_str().to_owned())
        .collect::<std::collections::BTreeSet<_>>();
    let committed = before
        .memories
        .iter()
        .map(|entry| entry.stable_key.clone())
        .collect();
    assert_eq!(requested, committed);
    assert!(
        before
            .memories
            .iter()
            .all(|entry| entry.capacity.wasm_pages == 0)
    );
    let parent = ic_platform::principal::ExternalPrincipal::from_slice(&[73]);
    Env::import(EnvData {
        record: EnvRecord {
            parent_pid: Some(parent),
            ..EnvRecord::default()
        },
    });
    let after = MemoryRegistryOps::allocation_snapshot().unwrap();
    let bindings = after
        .memories
        .iter()
        .find(|entry| entry.stable_key == "canic.core.runtime.bindings.v1")
        .unwrap();
    assert!(bindings.capacity.wasm_pages > 0);
    assert_eq!(Env::export().record.parent_pid, Some(parent));
    assert_eq!(MemoryRegistryOps::allocation_snapshot().unwrap(), after);
    for entry in &after.memories {
        assert_eq!(entry.capacity.bytes, entry.capacity.wasm_pages * 65_536);
    }
    let bytes = ic_wire::native::encode_response(METHOD_ID, &after, RESPONSE_LIMITS).unwrap();
    let decoded: MemoryAllocationsResponse =
        ic_wire::native::decode_response(METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    assert_eq!(decoded, after);
}

#[test]
fn observations_require_existing_bootstrap_and_preserve_warm_authority() {
    assert_eq!(
        MemoryRegistryOps::allocation_snapshot().unwrap_err().code(),
        crate::diagnostics::codes::STATE_INVALID
    );
    assert!(!MemoryRegistryOps::is_initialized().unwrap());
    memory::bootstrap(memory::MemoryBootstrap {
        requests: memory::requests().unwrap(),
        grants: memory::grants::framework_grants(Vec::new()).unwrap(),
        admission: None,
    })
    .unwrap();
    let before = MemoryRegistryOps::allocation_snapshot().unwrap();
    memory::runtime::with_runtime(|_| {
        assert!(matches!(
            memory::bootstrap(memory::MemoryBootstrap {
                requests: memory::requests().unwrap(),
                grants: memory::grants::framework_grants(Vec::new()).unwrap(),
                admission: None,
            }),
            Err(memory::MemoryBootstrapError::Access(
                memory::runtime::MemoryRuntimeAccessError::ReentrantAccess
            ))
        ));
        Ok(())
    })
    .unwrap();
    assert_eq!(before, MemoryRegistryOps::allocation_snapshot().unwrap());
}

#[test]
fn current_composition_cold_reopen_preserves_data_and_ids() {
    let backing = VectorMemory::default();
    let declarations =
        ic_storage::SealedDeclarationSnapshot::new(memory::requests().unwrap()).unwrap();
    let grants = memory::grants::framework_grants(Vec::new()).unwrap();
    let policy = memory::CanicStoragePolicy::new();
    let mut runtime = MemoryRuntime::new(backing.clone()).unwrap();
    runtime.bootstrap(&declarations, &grants, &policy).unwrap();
    let key = StableKey::parse("canic.core.runtime.bindings.v1").unwrap();
    let id = runtime.allocation_id(&key).unwrap();
    let rows = runtime.open_memory(&key).unwrap();
    rows.grow(1).unwrap();
    rows.write(0, b"retained");
    drop(rows);
    let before = backing.borrow().clone();
    drop(runtime);
    let mut reopened = MemoryRuntime::new(backing.clone()).unwrap();
    assert!(matches!(
        reopened.open_memory(&key),
        Err(ic_storage::RuntimeOpenError::NotBootstrapped)
    ));
    reopened.bootstrap(&declarations, &grants, &policy).unwrap();
    assert_eq!(reopened.allocation_id(&key), Some(id));
    let rows = reopened.open_memory(&key).unwrap();
    let mut bytes = [0; 8];
    rows.read(0, &mut bytes);
    assert_eq!(&bytes, b"retained");
    assert_eq!(*backing.borrow(), before);
}

#[derive(Clone, Default)]
struct RefusableMemory {
    bytes: VectorMemory,
    refuse: std::rc::Rc<std::cell::Cell<bool>>,
}
impl ic_storage::MemoryBacking for RefusableMemory {
    fn backing_identity(&self) -> ic_storage::BackingIdentity {
        ic_storage::MemoryBacking::backing_identity(&self.bytes)
    }
}
impl Memory for RefusableMemory {
    fn size(&self) -> u64 {
        self.bytes.size()
    }
    fn grow(&self, pages: u64) -> i64 {
        if self.refuse.get() {
            -1
        } else {
            self.bytes.grow(pages)
        }
    }
    fn read(&self, offset: u64, bytes: &mut [u8]) {
        self.bytes.read(offset, bytes);
    }
    fn write(&self, offset: u64, bytes: &[u8]) {
        self.bytes.write(offset, bytes);
    }
}

#[test]
fn refused_directory_commit_publishes_no_authority_and_retries() {
    let backing = RefusableMemory::default();
    let mut runtime = MemoryRuntime::new(backing.clone()).unwrap();
    let source = ic_storage::SealedDeclarationSnapshot::new(memory::requests().unwrap()).unwrap();
    let grants = memory::grants::framework_grants(Vec::new()).unwrap();
    let policy = memory::CanicStoragePolicy::new();
    let before = backing.bytes.borrow().clone();
    backing.refuse.set(true);
    assert!(matches!(
        runtime.bootstrap(&source, &grants, &policy),
        Err(RuntimeBootstrapError::Directory(
            ic_storage::DirectoryError::Growth
        ))
    ));
    assert!(!runtime.is_bootstrapped());
    assert_eq!(*backing.bytes.borrow(), before);
    backing.refuse.set(false);
    runtime.bootstrap(&source, &grants, &policy).unwrap();
    runtime
        .verify_authority(&source, memory::CANIC_CORE_MEMORY_AUTHORITY)
        .unwrap();
}

#[test]
fn refused_allocation_growth_keeps_data_and_exclusive_custody() {
    let backing = RefusableMemory::default();
    let mut runtime = MemoryRuntime::new(backing.clone()).unwrap();
    runtime
        .bootstrap(
            &ic_storage::SealedDeclarationSnapshot::new(memory::requests().unwrap()).unwrap(),
            &memory::grants::framework_grants(Vec::new()).unwrap(),
            &memory::CanicStoragePolicy::new(),
        )
        .unwrap();
    let key = StableKey::parse("canic.core.runtime.bindings.v1").unwrap();
    let rows = runtime.open_memory(&key).unwrap();
    let before = backing.bytes.borrow().clone();
    backing.refuse.set(true);
    assert!(matches!(
        rows.grow(1),
        Err(ic_storage::RuntimeGrowError::BackingRefused { .. })
    ));
    assert_eq!(rows.size(), 0);
    assert_eq!(*backing.bytes.borrow(), before);
    backing.refuse.set(false);
    rows.grow(1).unwrap();
    rows.write(0, b"retained");
    assert!(matches!(
        runtime.open_memory(&key),
        Err(ic_storage::RuntimeOpenError::AlreadyOpen { .. })
    ));
    drop(runtime);
    assert!(matches!(
        MemoryRuntime::new(backing.clone()),
        Err(ic_storage::RuntimeConstructionError::BackingAlreadyClaimed)
    ));
    let mut bytes = [0; 8];
    rows.read(0, &mut bytes);
    assert_eq!(&bytes, b"retained");
    drop(rows);
    assert!(MemoryRuntime::new(backing).is_ok());
}

#[test]
fn eligibility_capacity_refusal_preserves_store_and_typed_ops_failure() {
    use crate::{
        ops::storage::intent::IntentStoreOpsError,
        storage::stable::intent::ReceiptBackedIntentStore,
    };
    ReceiptBackedIntentStore::reserve_application_eligibility_capacity(1).unwrap();
    let before = MemoryRegistryOps::allocation_snapshot().unwrap();
    let required_records = u64::from(u32::MAX);
    let source =
        ReceiptBackedIntentStore::reserve_application_eligibility_capacity(required_records)
            .unwrap_err();
    assert!(matches!(
        source,
        ic_storage::RuntimeGrowError::CapacityExceeded
    ));
    assert_eq!(MemoryRegistryOps::allocation_snapshot().unwrap(), before);
    let internal: InternalError =
        IntentStoreOpsError::ApplicationReceiptEligibilityCapacityUnavailable {
            required_records,
            source,
        }
        .into();
    assert_eq!(
        internal.code(),
        crate::diagnostics::codes::CAPACITY_UNAVAILABLE
    );
    ReceiptBackedIntentStore::reserve_application_eligibility_capacity(1).unwrap();
    assert_eq!(MemoryRegistryOps::allocation_snapshot().unwrap(), before);
}
