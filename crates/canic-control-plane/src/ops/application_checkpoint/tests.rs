//! Native stable ownership, exact replay and delayed-acknowledgement regression.

use super::*;
use crate::ops::component_registry::caller_authority::RootCallerOps;
use crate::storage::stable::application_checkpoint::RootCheckpointStateRecord;
use canic_contracts::ids::{
    AppId, CallerInstallation, CanisterRole, CanonicalNetworkId, ComponentBinding,
    ComponentInstanceId, ComponentSpecId, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
    FleetRegistryAuthority, ManagedCanisterBinding, SubnetId,
};

#[test]
fn original_unit_storage_replay_and_release_fence_delayed_prepares() {
    let (root, mut members) = checkpoint_unit();
    let begin = RootCheckpointBeginRequest {
        root_install_id: root.install_id,
        operation_id: [5; 32],
        expected_sequence: 0,
        unit_digest: protocol::unit_digest(&members).unwrap(),
    };
    let request = RootCheckpointRequest {
        root_install_id: root.install_id,
        operation_id: begin.operation_id,
        sequence: 1,
    };
    let mut wrong_unit = begin.clone();
    wrong_unit.unit_digest = [0; 32];
    assert_conflict(RootCheckpointOps::begin(
        &root,
        &wrong_unit,
        members.clone(),
    ));
    assert!(RootCheckpointOps::status(&root).unwrap().is_none());
    let initial = RootCheckpointOps::begin(&root, &begin, members.clone()).unwrap();
    assert_conflict(RootCheckpointOps::begin(
        &root,
        &wrong_unit,
        members.clone(),
    ));
    assert_eq!(
        RootCheckpointOps::status(&root).unwrap(),
        Some(initial.clone())
    );
    assert_eq!(initial.participants, 2);
    assert_conflict(RootCallerOps::require_mutation_allowed(None));
    members.reverse();
    assert_eq!(
        RootCheckpointOps::begin(&root, &begin, members.clone()).unwrap(),
        initial
    );
    reload_store();
    let RootCheckpointStep {
        binding,
        expected: phase,
    } = RootCheckpointOps::next(&root, &request).unwrap().unwrap();
    assert_eq!(phase, ApplicationCheckpointPhase::Prepared);
    let partial = RootCheckpointOps::acknowledge(&root, &request, &binding, phase).unwrap();
    assert_eq!(partial.acknowledged, 1);
    assert_eq!(
        RootCheckpointOps::acknowledge(&root, &request, &binding, phase).unwrap(),
        partial
    );
    assert_eq!(
        RootCheckpointOps::release(&root, &request).unwrap().phase,
        ApplicationCheckpointPhase::Releasing
    );
    assert_conflict(RootCheckpointOps::acknowledge(
        &root, &request, &binding, phase,
    ));
    assert_conflict(RootCallerOps::require_mutation_allowed(None));
    for _ in 0..2 {
        let RootCheckpointStep {
            binding,
            expected: phase,
        } = RootCheckpointOps::next(&root, &request).unwrap().unwrap();
        assert_eq!(phase, ApplicationCheckpointPhase::Released);
        RootCheckpointOps::acknowledge(&root, &request, &binding, phase).unwrap();
    }
    let released = RootCheckpointOps::status(&root).unwrap().unwrap();
    assert_eq!(released.phase, ApplicationCheckpointPhase::Released);
    assert!(RootCheckpointOps::next(&root, &request).unwrap().is_none());
    assert_eq!(
        RootCheckpointOps::release(&root, &request).unwrap(),
        released
    );
    RootCheckpointOps::require_catalog_mutation_allowed().unwrap();
    let mut wrong = request.clone();
    wrong.root_install_id = [9; 32];
    assert_conflict(RootCheckpointOps::release(&root, &wrong));
    let mut changed = members.clone();
    changed[0].receiver.install_id = [8; 32];
    assert_conflict(RootCheckpointOps::begin(&root, &begin, changed));
    let next = RootCheckpointBeginRequest {
        root_install_id: root.install_id,
        operation_id: [6; 32],
        expected_sequence: 1,
        unit_digest: begin.unit_digest,
    };
    assert_eq!(
        RootCheckpointOps::begin(&root, &next, members.clone())
            .unwrap()
            .sequence,
        2
    );
    assert_conflict(RootCheckpointOps::begin(&root, &begin, members));
    assert_conflict(RootCheckpointOps::acknowledge(
        &root, &request, &binding, phase,
    ));
}

fn managed_component_binding() -> ManagedCanisterBinding {
    let fleet = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([43; 32]),
        },
        app: AppId::from("root-admission-test"),
    };
    let placement_subnet = SubnetId::from_principal(
        ic_platform::principal::ExternalPrincipal::from_slice(&[44; 29]),
    );
    ManagedCanisterBinding::Component(ComponentBinding {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet,
                coordinator_subnet: SubnetId::from_principal(
                    ic_platform::principal::ExternalPrincipal::from_slice(&[45; 29]),
                ),
                coordinator: ic_platform::principal::ExternalPrincipal::from_slice(&[46; 29]),
                recovery_controllers: Vec::new(),
            },
            epoch: 1,
        },
        component: ComponentInstanceId::from_generated_bytes([47; 32]),
        component_spec: ComponentSpecId::try_from(String::from("app")).expect("Component Spec ID"),
        spec_hash: [48; 32],
        role: CanisterRole::from("app"),
        placement_subnet,
        fleet_subnet_root: ic_platform::principal::ExternalPrincipal::from_slice(&[49; 29]),
        canister_id: ic_platform::principal::ExternalPrincipal::from_slice(&[50; 29]),
    })
}

fn assert_conflict<T>(result: Result<T, InternalError>) {
    assert_eq!(
        result.err().map(|error| error.code()),
        Some(InternalError::conflict().code())
    );
}

fn reload_store() {
    let original = RootCheckpointStore::read().unwrap();
    let bytes = canic_contracts::serialization::serialize(&RootCheckpointStateRecord {
        current: Some(original),
    })
    .unwrap();
    let decoded: RootCheckpointStateRecord =
        canic_contracts::serialization::deserialize(&bytes).unwrap();
    RootCheckpointStore::write(decoded.current.unwrap()).unwrap();
}

fn checkpoint_unit() -> (CallerRootAuthority, Vec<CallerReceiverAuthority>) {
    let mut first = CallerInstallation {
        binding: managed_component_binding(),
        install_id: [1; 32],
        component_install_id: [2; 32],
    };
    let root = CallerRootAuthority {
        registry: first.component().authority.clone(),
        root: first.component().fleet_subnet_root,
        install_id: [3; 32],
    };
    let mut members = vec![CallerReceiverAuthority {
        receiver: first.clone(),
        issuer: root.clone(),
        policy_digest: [4; 32],
    }];
    let ManagedCanisterBinding::Component(component) = &mut first.binding else {
        unreachable!()
    };
    component.canister_id = ic_platform::principal::ExternalPrincipal::from_slice(&[51; 29]);
    members.push(CallerReceiverAuthority {
        receiver: first,
        issuer: root.clone(),
        policy_digest: [4; 32],
    });
    members.sort_by_key(|member| member.receiver.canister());
    (root, members)
}
