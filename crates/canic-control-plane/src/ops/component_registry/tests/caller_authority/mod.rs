//! Module: ops::component_registry::tests::caller_authority
//!
//! Original-census publication recovery and capacity boundaries in the Registry owner.

use super::*;
use crate::{
    ops::component_registry::caller_authority::{CallerReceiverPlan, RootCallerOps},
    storage::stable::component_registry::caller_authority::{
        CallerJournalKey, CallerJournalPhase, CallerJournalRowRecord, CallerLifecycleScope,
    },
};
use canic_contracts::{
    dto::caller_authority::{
        CallerAuthorityChange, CallerAuthorityPhase, CallerAuthorityReceipt, CallerAuthorityStatus,
    },
    ids::{CallerInstallation, CallerReceiverAuthority, CallerRootAuthority},
};
use canic_core::control_plane_support::ops::caller_authority::CallerAuthorityOps;

pub(super) fn assert_current_child_runtime_uses_registered_allocation(
    original: &RootComponentRegistryData,
) {
    let current = original.child_allocations.first().unwrap();
    let RootComponentChildAllocationProgressRecord::Committed { installation, .. } =
        &current.progress
    else {
        panic!("committed child allocation fixture");
    };
    let binding = ManagedCanisterBinding::ComponentChild(installation.binding.clone());
    let mut data = original.clone();
    let mut historical = current.clone();
    historical.operation_id = [0xb3; 32];
    assert_ne!(historical.operation_id, current.operation_id);
    data.child_allocations.push(historical);
    RootComponentRegistryStore::import(data.clone());
    assert_eq!(
        ComponentRegistryOps::managed_runtime_operation_id(&binding).unwrap(),
        current.operation_id
    );
    data.child_allocations
        .retain(|record| record.operation_id != current.operation_id);
    RootComponentRegistryStore::import(data);
    assert_eq!(
        ComponentRegistryOps::managed_runtime_operation_id(&binding)
            .unwrap_err()
            .public_code(),
        InternalError::invariant().public_code()
    );
    RootComponentRegistryStore::import(original.clone());
}

fn receiver(id: u8) -> CallerReceiverAuthority {
    let root = root_binding();
    let receiver = CallerInstallation {
        binding: ManagedCanisterBinding::Component(ComponentBinding {
            authority: root.authority.clone(),
            component: ComponentInstanceId::from_generated_bytes([id; 32]),
            component_spec: "projects".parse().unwrap(),
            spec_hash: [6; 32],
            role: CanisterRole::from("project_hub"),
            placement_subnet: root.placement_subnet,
            fleet_subnet_root: root.fleet_subnet_root,
            canister_id: Principal::from_slice(&[id; 29]),
        }),
        install_id: [id; 32],
        component_install_id: [id; 32],
    };
    CallerReceiverAuthority {
        issuer: CallerRootAuthority {
            registry: root.authority,
            root: root.fleet_subnet_root,
            install_id: [19; 32],
        },
        receiver,
        policy_digest: [20; 32],
    }
}

fn plan(authority: CallerReceiverAuthority) -> CallerReceiverPlan {
    CallerReceiverPlan {
        authority,
        base_generation: 0,
        enroll: true,
        retire: false,
        before: Vec::new(),
        after: vec![CallerAuthorityChange::OpenReceiver],
    }
}

fn status(authority: CallerReceiverAuthority) -> CallerAuthorityStatus {
    CallerAuthorityStatus {
        readiness: canic_contracts::dto::caller_authority::CallerAuthorityReadiness::FrameworkReady,
        authority,
        generation: 0,
        open: false,
        retired: false,
        entries: 0,
        reserved_bytes: 8192,
        pending_operation: None,
        receipt: None,
    }
}

fn assert_conflict(error: InternalError) {
    assert_eq!(
        canic_contracts::dto::error::Error::from(error),
        canic_contracts::dto::error::Error::from_registered(
            canic_contracts::diagnostics::codes::STATE_CONFLICT
        )
    );
}

#[test]
fn caller_publication_keeps_original_census_through_missing_receivers_and_cold_restart() {
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
    let first = receiver(30);
    let second = receiver(31);
    let scope = CallerLifecycleScope::ActivateComponent(first.receiver.clone());
    let original = RootCallerOps::begin(
        [30; 32],
        first.issuer.clone(),
        scope.clone(),
        vec![plan(first.clone()), plan(second.clone())],
    )
    .unwrap();
    assert_conflict(
        RootCallerOps::begin(
            [30; 32],
            first.issuer.clone(),
            scope,
            vec![plan(first.clone())],
        )
        .unwrap_err(),
    );
    RootCallerOps::reserve([30; 32], &status(first.clone()), 8, 65_536).unwrap();
    let reserved = restart_component_registry();
    RootCallerOps::restore(&first.issuer).unwrap();
    assert_conflict(RootCallerOps::require_mutation_allowed(Some([99; 32])).unwrap_err());
    assert!(RootCallerOps::mark_membership_committed([30; 32]).is_err());
    assert!(RootCallerOps::reserve([30; 32], &status(second.clone()), 1, 8_192).is_err());
    assert_eq!(RootComponentRegistryStore::export(), reserved);
    RootCallerOps::reserve([30; 32], &status(second), 1, 16_384).unwrap();
    assert_eq!(
        RootCallerOps::next_phase([30; 32]).unwrap().phase,
        CallerJournalPhase::Preparing
    );
    for _ in 0..2 {
        RootCallerOps::advance_recipient([30; 32]).unwrap();
    }
    assert_eq!(
        RootCallerOps::next_phase([30; 32]).unwrap().phase,
        CallerJournalPhase::Prepared
    );
    RootCallerOps::mark_membership_committed([30; 32]).unwrap();
    for ordinal in 0..2 {
        let step = RootCallerOps::step([30; 32], ordinal, 0).unwrap();
        let receipt = CallerAuthorityReceipt {
            publication: CallerAuthorityOps::publication_to_dto(step.publication),
            phase: CallerAuthorityPhase::Complete,
        };
        let mut committed = receipt.clone();
        committed.phase = CallerAuthorityPhase::Committed;
        RootCallerOps::acknowledge([30; 32], ordinal, 0, committed).unwrap();
        let mut delayed = receipt.clone();
        delayed.phase = CallerAuthorityPhase::Prepared;
        RootCallerOps::acknowledge([30; 32], ordinal, 0, delayed).unwrap();
        assert_eq!(RootCallerOps::step([30; 32], ordinal, 0).unwrap().phase,
            Some(canic_core::control_plane_support::model::caller_authority::CallerReceiptPhase::Committed));
        RootCallerOps::acknowledge([30; 32], ordinal, 0, receipt.clone()).unwrap();
        RootCallerOps::acknowledge([30; 32], ordinal, 0, receipt).unwrap();
        RootCallerOps::advance_recipient([30; 32]).unwrap();
    }
    assert_eq!(
        RootCallerOps::next_phase([30; 32]).unwrap().phase,
        CallerJournalPhase::Published
    );
    assert!(RootCallerOps::complete([30; 32]).is_err());
    for ordinal in 0..2 {
        RootCallerOps::mark_startup_released([30; 32], ordinal).unwrap();
    }
    RootCallerOps::complete([30; 32]).unwrap();
    restart_component_registry();
    RootCallerOps::restore(&first.issuer).unwrap();
    RootCallerOps::require_mutation_allowed(None).unwrap();
    let complete = RootCallerOps::operation([30; 32]).unwrap();
    assert_eq!(complete.census_hash, original.census_hash);
    assert_eq!(complete.recipient_count, original.recipient_count);
    assert_eq!(
        RootCallerOps::receiver(first.receiver.canister())
            .unwrap()
            .generation,
        1
    );
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

#[test]
fn caller_publication_cold_restore_rejects_changed_steps_and_issuing_installation() {
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
    let receiver = receiver(40);
    RootCallerOps::begin(
        [40; 32],
        receiver.issuer.clone(),
        CallerLifecycleScope::ActivateComponent(receiver.receiver.clone()),
        vec![plan(receiver.clone())],
    )
    .unwrap();
    let mut wrong_issuer = receiver.issuer.clone();
    wrong_issuer.install_id = [99; 32];
    assert_conflict(RootCallerOps::restore(&wrong_issuer).unwrap_err());
    let snapshot = RootComponentRegistryStore::export();
    let mut step = RootCallerOps::step([40; 32], 0, 0).unwrap();
    step.publication.generation = 99;
    RootComponentRegistryStore::caller_replace(
        CallerJournalKey::Step {
            operation: [40; 32],
            recipient: 0,
            ordinal: 0,
        },
        CallerJournalRowRecord::Step(Box::new(step)),
    )
    .unwrap();
    assert_conflict(RootCallerOps::restore(&receiver.issuer).unwrap_err());
    RootComponentRegistryStore::import(snapshot);
    RootCallerOps::restore(&receiver.issuer).unwrap();
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}

#[test]
fn caller_publication_compacts_terminal_progress_in_bounded_cold_restorable_pages() {
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
    let receivers: Vec<_> = (50..90).map(receiver).collect();
    let operation = [50; 32];
    let original = RootCallerOps::begin(
        operation,
        receivers[0].issuer.clone(),
        CallerLifecycleScope::ActivateComponent(receivers[0].receiver.clone()),
        receivers.iter().cloned().map(plan).collect(),
    )
    .unwrap();
    for receiver in &receivers {
        RootCallerOps::reserve(operation, &status(receiver.clone()), 1, 16_384).unwrap();
    }
    RootCallerOps::next_phase(operation).unwrap();
    for _ in &receivers {
        RootCallerOps::advance_recipient(operation).unwrap();
    }
    RootCallerOps::next_phase(operation).unwrap();
    RootCallerOps::mark_membership_committed(operation).unwrap();
    for ordinal in 0..original.recipient_count {
        let step = RootCallerOps::step(operation, ordinal, 0).unwrap();
        RootCallerOps::acknowledge(
            operation,
            ordinal,
            0,
            CallerAuthorityReceipt {
                publication: CallerAuthorityOps::publication_to_dto(step.publication),
                phase: CallerAuthorityPhase::Complete,
            },
        )
        .unwrap();
        RootCallerOps::advance_recipient(operation).unwrap();
    }
    RootCallerOps::next_phase(operation).unwrap();
    for ordinal in 0..original.recipient_count {
        RootCallerOps::mark_startup_released(operation, ordinal).unwrap();
    }
    RootCallerOps::complete(operation).unwrap();
    let before = RootComponentRegistryStore::caller_row_count();
    assert!(!RootCallerOps::compact(operation).unwrap());
    assert_eq!(RootComponentRegistryStore::caller_row_count(), before - 64);
    restart_component_registry();
    RootCallerOps::restore(&receivers[0].issuer).unwrap();
    assert!(RootCallerOps::compact(operation).unwrap());
    restart_component_registry();
    RootCallerOps::restore(&receivers[0].issuer).unwrap();
    let retained = RootCallerOps::operation(operation).unwrap();
    assert_eq!(retained.phase, CallerJournalPhase::Compacted);
    assert_eq!(retained.census_hash, original.census_hash);
    assert_eq!(retained.scope, original.scope);
    let compacted = RootComponentRegistryStore::export();
    assert!(RootCallerOps::compact(operation).unwrap());
    assert_eq!(RootComponentRegistryStore::export(), compacted);
    RootComponentRegistryStore::import(RootComponentRegistryData::default());
}
