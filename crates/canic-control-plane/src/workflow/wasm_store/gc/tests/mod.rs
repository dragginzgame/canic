//! Bounded fixture transactions preserve the existing ledger and clearing fence.

use super::*;
use crate::storage::stable::{
    fixture_store::{FixtureStore, FixtureStoreData},
    template::WasmStoreGcStateStore,
};

struct StoreScope;

impl Drop for StoreScope {
    fn drop(&mut self) {
        FixtureStore::import(FixtureStoreData::default());
        WasmStoreGcStateStore::clear_for_test();
    }
}

#[test]
fn batches_preserve_exact_accounting_and_resume_after_restoration() {
    let _scope = StoreScope;
    FixtureStore::import(FixtureStoreData::default());
    WasmStoreGcStateStore::clear_for_test();
    let entries = (0..33)
        .map(|index| FixtureStore::chunk_entry([1; 32], index, vec![7; 128]))
        .collect::<Vec<_>>();
    let bytes = FixtureStore::projected_bytes(&entries).unwrap();
    FixtureStore::commit(entries, bytes);
    let original = FixtureStore::export();
    assert_eq!(
        clear_fixture_batch(),
        Err(Error::from_registered(
            canic_contracts::diagnostics::codes::STATE_CONFLICT
        ))
    );
    assert_eq!(FixtureStore::export(), original);
    WasmStoreGcOps::prepare([1; 32], 1).unwrap();
    WasmStoreGcOps::begin(2).unwrap();
    WasmStoreGcOps::begin_clearing(3).unwrap();

    for remaining in [17, 1] {
        assert!(!clear_fixture_batch().unwrap());
        assert_eq!(FixtureStore::inventory().stored_chunks, remaining);
        let retained = FixtureStore::export();
        let exact_bytes: u64 = retained
            .entries
            .iter()
            .map(|entry| (entry.key.len() + entry.value.len()) as u64)
            .sum();
        assert_eq!(FixtureStore::occupied_bytes(), exact_bytes);
        let gc = WasmStoreGcStateStore::export();
        FixtureStore::import(retained.clone());
        WasmStoreGcStateStore::import(gc);
        assert_eq!(FixtureStore::export(), retained);
        assert!(WasmStoreGcOps::require_collection([1; 32]).unwrap());
    }
    assert!(clear_fixture_batch().unwrap());
    assert_eq!(FixtureStore::occupied_bytes(), 0);
    assert_eq!(FixtureStore::export(), FixtureStoreData::default());
    assert!(clear_fixture_batch().unwrap());
    assert_eq!(FixtureStore::occupied_bytes(), 0);
}

#[test]
fn failed_collection_reports_failure_without_starting_an_independent_retry() {
    let result = collection_completion(Err(Error::from_registered(
        canic_contracts::diagnostics::codes::STATE_CONFLICT,
    )));
    assert_eq!(result.completion(), TimerCompletion::retryable_failure(0));
    assert_eq!(result.decision(), OnceDecision::Stop);
    let result = collection_completion(Ok(()));
    assert_eq!(result.completion(), TimerCompletion::success(1));
    assert_eq!(result.decision(), OnceDecision::Stop);
}
