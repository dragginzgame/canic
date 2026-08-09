//! Focused durable-cursor qualification for root Directory synchronization.

use super::*;
use crate::storage::stable::component_provisioning::{
    RootComponentProvisioningData, RootComponentProvisioningStore,
};
use canic_core::{
    control_plane_support::error::InternalErrorClass,
    dto::{
        component_provisioning::RootComponentDirectorySynchronizationRequest,
        component_registry::ComponentRegistryHead, fleet_registry::FleetRegistryVersion,
    },
    ids::{
        AppId, CanonicalNetworkId, ComponentInstanceId, FleetBinding, FleetCoordinatorBinding,
        FleetId, FleetKey, FleetRegistryAuthority, SubnetId,
    },
};

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn authority() -> FleetRegistryAuthority {
    FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([1; 32]),
                },
                app: AppId::from("directory_sync_test"),
            },
            coordinator_subnet: SubnetId::from_principal(principal(2)),
            coordinator: principal(3),
        },
        epoch: 1,
    }
}

fn request(operation_id: [u8; 32]) -> RootComponentDirectorySynchronizationRequest {
    let authority = authority();
    RootComponentDirectorySynchronizationRequest {
        operation_id,
        plan_hash: [5; 32],
        source_fleet_registry: FleetRegistryVersion {
            authority: authority.clone(),
            revision: 7,
            content_hash: [7; 32],
        },
        published_fleet_registry: FleetRegistryVersion {
            authority,
            revision: 8,
            content_hash: [8; 32],
        },
        expected_synchronized_component_count: 0,
    }
}

fn target() -> RootComponentDirectorySynchronizationTargetView {
    let component = ComponentInstanceId::from_generated_bytes([11; 32]);
    RootComponentDirectorySynchronizationTargetView {
        component,
        canister_id: principal(12),
        allocation_operation_id: [13; 32],
        source_registry: ComponentRegistryHead {
            component,
            revision: 4,
            content_hash: [14; 32],
        },
    }
}

fn intent() -> RootComponentDirectorySynchronizationIntentView {
    let target = target();
    RootComponentDirectorySynchronizationIntentView {
        component_index: 0,
        component: target.component,
        canister_id: target.canister_id,
        allocation_operation_id: target.allocation_operation_id,
        previous_registry: target.source_registry,
        registry: ComponentRegistryHead {
            component: target.component,
            revision: 5,
            content_hash: [15; 32],
        },
        directory_synchronized_at_ns: 200,
        directory_authority_hash: [16; 32],
        started_at_ns: 200,
    }
}

#[test]
fn synchronization_journals_intent_reconciles_and_replays_terminal_receipt() {
    RootComponentProvisioningStore::import(RootComponentProvisioningData::default());
    let command = request([4; 32]);
    let accepted = RootComponentDirectorySynchronizationOps::accept(
        &command,
        principal(9),
        [10; 32],
        vec![target()],
        100,
    )
    .expect("accept synchronization authority");
    assert_eq!(accepted.affected_component_count, 1);
    assert!(!accepted.complete);

    let other = RootComponentDirectorySynchronizationOps::accept(
        &request([20; 32]),
        principal(9),
        [10; 32],
        vec![],
        101,
    )
    .expect_err("a second synchronization cannot overlap");
    assert_eq!(other.class(), InternalErrorClass::Domain);

    let intent = intent();
    let first =
        RootComponentDirectorySynchronizationOps::advance(&command, Some(intent.clone()), 200)
            .expect("persist first target intent");
    assert_eq!(
        first,
        RootComponentDirectorySynchronizationDisposition::Invoke(intent.clone())
    );
    let replay = RootComponentDirectorySynchronizationOps::advance(&command, None, 201)
        .expect("reconcile retained target intent");
    assert_eq!(
        replay,
        RootComponentDirectorySynchronizationDisposition::Reconcile(intent.clone())
    );

    let terminal =
        RootComponentDirectorySynchronizationOps::record_synchronized(&command, &intent, 202)
            .expect("record exact target evidence");
    assert!(terminal.complete);
    assert_eq!(terminal.synchronized_component_count, 1);
    assert_eq!(terminal.synchronized_at_ns, Some(202));
    assert_ne!(terminal.receipt_content_hash, [0; 32]);
    assert_eq!(
        RootComponentProvisioningStore::state().active_directory_synchronization_operation_id,
        None
    );

    let replay = RootComponentDirectorySynchronizationOps::advance(&command, None, 203)
        .expect("replay terminal synchronization");
    assert_eq!(
        replay,
        RootComponentDirectorySynchronizationDisposition::Current(Box::new(terminal))
    );
}

#[test]
fn synchronization_exact_retry_rejects_changed_authority() {
    RootComponentProvisioningStore::import(RootComponentProvisioningData::default());
    let request = request([21; 32]);
    let first = RootComponentDirectorySynchronizationOps::accept(
        &request,
        principal(9),
        [10; 32],
        vec![],
        100,
    )
    .expect("accept empty synchronization");
    let exact = RootComponentDirectorySynchronizationOps::accept(
        &request,
        principal(9),
        [10; 32],
        vec![],
        100,
    )
    .expect("replay exact acceptance");
    assert_eq!(exact, first);

    let mut conflicting = request;
    conflicting.plan_hash = [22; 32];
    let error = RootComponentDirectorySynchronizationOps::status(&conflicting)
        .expect_err("changed plan hash must reject");
    assert_eq!(error.class(), InternalErrorClass::Domain);
}
