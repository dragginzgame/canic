//! Initial capacity ownership survives restart and opens only after the exact import settles.

use super::*;
use crate::ops::canister_pool::{
    CanisterPoolOps,
    capacity_import::tests::{config, finish, identity, principal, reservation},
};
use canic_core::cdk::structures::Storable;
use std::borrow::Cow;

fn held() {
    CanisterPoolStore::clear();
    let mut state = CanisterPoolStore::state();
    state.bootstrap_import = Some(PoolImportBootstrapRecord {
        review_sha256: [43; 32],
        install_id: [44; 32],
        store: principal(9),
        operator: principal(2),
        sources: vec![principal(8)],
    });
    CanisterPoolStore::set_state(state);
}

#[test]
fn bootstrap_capacity_hold_allows_only_supplied_store_and_fences_allocation() {
    held();
    let original = CanisterPoolStore::state();
    let bytes = original.to_bytes().into_owned();
    CanisterPoolStore::set_state(CanisterPoolStateRecord::from_bytes(Cow::Owned(bytes)));
    assert_eq!(CanisterPoolStore::state(), original);
    assert!(CanisterPoolImportOps::is_active());
    assert_conflict(
        crate::ops::component_provisioning::RootComponentProvisioningOps::require_acceptance_open(
            [1; 32],
        ),
    );
    assert_conflict(crate::ops::component_provisioning::RootComponentProvisioningOps::require_root_draining_open());
    assert_conflict(CanisterPoolImportOps::require_idle());
    assert_conflict(CanisterPoolOps::initialize_store(principal(7), 1));
    CanisterPoolOps::initialize_store(principal(9), 1).unwrap();
    assert_conflict(CanisterPoolOps::initialize_imports(
        &config(),
        &[principal(8)],
        1,
    ));
    assert_eq!(CanisterPoolStore::state(), original);
    assert!(CanisterPoolStore::get(&principal(8)).is_none());
}

#[test]
fn bootstrap_capacity_hold_rejects_substituted_reservation_without_consuming_sequence() {
    held();
    let before = CanisterPoolStore::state();
    let mut wrong = reservation();
    wrong.sources[0].canister_id = principal(7);
    assert_conflict(CanisterPoolImportOps::reserve(wrong, &config(), 1));
    let mut wrong = reservation();
    wrong.operator = principal(6);
    wrong.transitional_controllers = vec![principal(1), principal(4), principal(6)];
    assert_conflict(CanisterPoolImportOps::reserve(wrong, &config(), 1));
    assert_eq!(CanisterPoolStore::state(), before);
}

#[test]
fn bootstrap_capacity_hold_releases_only_after_settlement_and_publication() {
    held();
    CanisterPoolOps::initialize_store(principal(9), 1).unwrap();
    CanisterPoolImportOps::reserve(reservation(), &config(), 1).unwrap();
    assert_conflict(CanisterPoolOps::initialize_store(principal(9), 1));
    assert!(CanisterPoolImportOps::release(identity(), [5; 32]).is_err());
    assert!(pending());
    finish();
    assert!(CanisterPoolImportOps::release(identity(), [5; 32]).is_err());
    assert!(pending());
    CanisterPoolImportOps::settle(identity(), 9_980, 990).unwrap();
    assert!(pending());
    assert_conflict(CanisterPoolImportOps::require_idle());
    let released = CanisterPoolImportOps::release(identity(), [5; 32]).unwrap();
    assert!(!pending());
    CanisterPoolImportOps::require_idle().unwrap();
    assert_eq!(
        CanisterPoolImportOps::release(identity(), [5; 32]).unwrap(),
        released
    );
}

fn assert_conflict<T: std::fmt::Debug>(result: Result<T, InternalError>) {
    assert_eq!(result.unwrap_err().code(), InternalError::conflict().code());
}
