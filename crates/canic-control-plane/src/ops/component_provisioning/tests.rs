//! Focused proofs for durable root-batch acceptance and protected member derivation.

use super::*;
use crate::storage::stable::component_provisioning::{
    RootComponentProvisioningData, RootComponentProvisioningStore,
};
use crate::view::component_registry::{
    ComponentRegistryPartitionView, RootComponentAllocationProgressView,
    RootComponentCommitmentView, RootComponentCreationEffectView, RootComponentInstallEffectView,
};
use candid::Principal;
use canic_core::{
    bootstrap::parse_config_model,
    cdk::types::Cycles,
    control_plane_support::{config::ComponentTopology, model::replay::ReplayCostGuardSettlement},
    dto::{
        component_provisioning::{
            ComponentGroupPlacementPlan, ComponentGroupPlanEntry, FleetSubnetRootProvisioningBatch,
            RootComponentActivationRequest, RootComponentPublicationRequest,
        },
        component_registry::{
            ComponentLifecycleStatus, ComponentProvisioningOrigin, ComponentRegistryHead,
        },
        fleet_registry::{
            FleetDirectoryProvenance, FleetDirectorySnapshot, FleetRegistryVersion,
            FleetSubnetRootDirectoryEntry, FleetSubnetRootStatus,
        },
    },
    ids::{
        AppId, CanisterRole, CanonicalNetworkId, ComponentDeploymentConfigurationDigest,
        ComponentGroupMemberPath, ComponentInstanceId, ComponentSpecAdmission, CyclesFundingBudget,
        FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority,
        FleetSubnetCanisterPoolConfig, FleetSubnetRootBinding, FleetSubnetRootLimits,
        FleetSubnetRootReleaseSet, IntentId, ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest,
        SubnetId,
    },
};
use std::collections::{BTreeMap, BTreeSet};

const CONFIG: &str = r#"
[app]
name = "root_batch_test"

[roles.root]
kind = "root"
package = "root"

[roles.alpha]
kind = "canister"
package = "alpha"

[component_specs.alpha]
component_role = "alpha"
maximum_instances = 8

[component_groups.cell.components.alpha]
component_spec = "alpha"
labels = { tier = "api" }

[component_group_deployments.cells]
component_group = "cell"
initial_placements = 1
maximum_placements = 4
placement.maximum_per_root = 2
placement.minimum_distinct_roots = 1
"#;

struct Fixture {
    request: RootComponentProvisioningAcceptanceRequest,
    validation: RootComponentProvisioningBatchValidation,
}

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(principal(byte))
}

fn authority() -> FleetRegistryAuthority {
    FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([3; 32]),
                },
                app: AppId::from("root_batch_test"),
            },
            coordinator_subnet: subnet(4),
            coordinator: principal(5),
        },
        epoch: 1,
    }
}

fn root_binding(
    topology: &ComponentTopology,
    component_spec: &canic_core::ids::ComponentSpecId,
) -> FleetSubnetRootBinding {
    let spec = topology.get(component_spec).expect("Component Spec");
    let component_admissions = vec![ComponentSpecAdmission {
        component_spec: component_spec.clone(),
        spec_hash: spec.spec_hash,
        maximum_root_instances: 8,
    }];
    let projection = topology
        .project_for_admissions(&component_admissions)
        .expect("root topology projection");
    FleetSubnetRootBinding {
        authority: authority(),
        placement_subnet: subnet(6),
        fleet_subnet_root: principal(7),
        component_admissions,
        component_topology_digest: projection.digest().expect("topology digest"),
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 8,
            maximum_registry_bytes: 16_777_216,
            maximum_wasm_store_bytes: 40_000_000,
            canister_pool: FleetSubnetCanisterPoolConfig {
                minimum_size: 1,
                maximum_size: 8,
                canister_cycles: Cycles::new(5_000_000_000_000),
            },
            cycles_funding: CyclesFundingBudget {
                window_secs: 3_600,
                maximum_cycles: Cycles::new(10_000_000_000_000),
            },
            maximum_group_placements: 4,
        },
    }
}

fn fixture() -> Fixture {
    RootComponentProvisioningStore::import(RootComponentProvisioningData::default());
    let config = parse_config_model(CONFIG).expect("valid config");
    let topology = config
        .compile_component_topology()
        .expect("Component Topology");
    let deployments = config
        .compile_component_group_deployment_topology()
        .expect("deployment topology");
    let deployment = deployments
        .get(&"cells".parse().expect("deployment ID"))
        .expect("deployment");
    let member = deployment.members.first().expect("flattened member");
    let root = root_binding(&topology, &member.component_spec);
    let entry = ComponentGroupPlanEntry {
        member_path: member.member_path.clone(),
        component_spec: member.component_spec.clone(),
        spec_hash: member.component_spec_hash,
        purpose: member.purpose.clone(),
        labels: member.labels.clone(),
        limits: member.limits.clone(),
    };
    let batch = FleetSubnetRootProvisioningBatch {
        root: root.clone(),
        active_release_set: FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        },
        placements: vec![ComponentGroupPlacementPlan {
            group_placement: canic_core::ids::ComponentGroupPlacementId {
                deployment: deployment.deployment.clone(),
                ordinal: 0,
            },
            component_group: deployment.component_group.clone(),
            entries: vec![entry],
        }],
    };
    Fixture {
        request: RootComponentProvisioningAcceptanceRequest {
            fleet_registry: FleetRegistryVersion {
                authority: root.authority,
                revision: 3,
                content_hash: [10; 32],
            },
            configuration_digest: config
                .compile_component_deployment_configuration_digest()
                .expect("configuration digest"),
            operation_id: [11; 32],
            plan_hash: [12; 32],
            batch,
        },
        validation: RootComponentProvisioningBatchValidation {
            placement_count: 1,
            component_count: 1,
            component_spec_counts: BTreeMap::from([(member.component_spec.clone(), 1)]),
            component_roles: BTreeSet::from([topology
                .get(&member.component_spec)
                .expect("Component Spec")
                .component_role
                .clone()]),
        },
    }
}

fn claimed_allocation(
    mut allocation: RootComponentAllocationView,
    canister: Principal,
    root: Principal,
) -> RootComponentAllocationView {
    allocation.progress = RootComponentAllocationProgressView::Created {
        effect: RootComponentCreationEffectView {
            wasm_store: principal(30),
            payload_hash: [32; 32],
            payload_size_bytes: 1,
            initial_cycles: Cycles::new(5_000_000_000_000),
            controller: root,
            cost_guard_settlement: ReplayCostGuardSettlement {
                quota_intent_id: IntentId(1),
                reservation_intent_id: IntentId(2),
            },
            charged_entry_bytes: 1,
        },
        canister,
    };
    allocation
}

fn installed_allocation(
    mut allocation: RootComponentAllocationView,
    root: &FleetSubnetRootBinding,
) -> RootComponentAllocationView {
    let RootComponentAllocationProgressView::Created { effect, canister } = allocation.progress
    else {
        panic!("claimed allocation must be Created")
    };
    allocation.progress = RootComponentAllocationProgressView::Verified {
        creation: effect,
        canister,
        installation: RootComponentInstallEffectView {
            raw_module_hash: [33; 32],
            chunk_hashes: vec![vec![34; 32]],
            binding: canic_core::ids::ComponentBinding {
                authority: root.authority.clone(),
                component: allocation.component,
                component_spec: allocation.component_spec.clone(),
                spec_hash: allocation.spec_hash,
                role: allocation.role.clone(),
                placement_subnet: root.placement_subnet,
                fleet_subnet_root: root.fleet_subnet_root,
                canister_id: canister,
            },
            cost_guard_settlement: ReplayCostGuardSettlement {
                quota_intent_id: IntentId(3),
                reservation_intent_id: IntentId(4),
            },
            charged_entry_bytes: 1,
        },
    };
    allocation
}

fn committed_allocation(
    mut allocation: RootComponentAllocationView,
) -> (RootComponentAllocationView, ComponentRegistryPartitionView) {
    let RootComponentAllocationProgressView::Verified {
        creation,
        canister,
        installation,
    } = allocation.progress
    else {
        panic!("installed allocation must be Verified")
    };
    let registry = ComponentRegistryHead {
        component: allocation.component,
        revision: 1,
        content_hash: [35; 32],
    };
    let partition = ComponentRegistryPartitionView {
        binding: installation.binding.clone(),
        provisioning_origin: allocation.provisioning_origin.clone(),
        release_set: allocation.release_set,
        status: ComponentLifecycleStatus::Prepared,
        revision: registry.revision,
        content_hash: registry.content_hash,
        descendant_content_hash: [36; 32],
        directory_synchronized_at_ns: 200,
        reserved_descendants: 0,
        committed_descendants: 0,
        encoded_bytes: 1,
    };
    allocation.progress = RootComponentAllocationProgressView::Committed {
        creation,
        canister,
        installation,
        commitment: RootComponentCommitmentView {
            registry,
            prepared_registry_encoded_bytes: partition.encoded_bytes,
            directory_synchronized_at_ns: partition.directory_synchronized_at_ns,
            directory_authority_hash: [37; 32],
            directory_prepared: false,
            runtime_activated: false,
            membership: None,
        },
    };
    (allocation, partition)
}

fn assert_group_context(
    context: ProtectedComponentDeployment,
    canister: Principal,
    configuration_digest: ComponentDeploymentConfigurationDigest,
) {
    let ProtectedComponentDeployment::GroupMember {
        binding,
        configuration_digest: actual_digest,
        ..
    } = context
    else {
        panic!("group provisioning must derive grouped context")
    };
    assert_eq!(binding.canister_id, canister);
    assert_eq!(actual_digest, configuration_digest);
}

fn reserve_single_member(
    fixture: &Fixture,
) -> (
    RootComponentProvisioningAdvanceRequest,
    RootComponentAllocationView,
    RootComponentProvisioningView,
) {
    let accepted =
        RootComponentProvisioningOps::accept(fixture.request.clone(), &fixture.validation, 100)
            .expect("accept batch");
    let request = RootComponentProvisioningAdvanceRequest {
        operation_id: fixture.request.operation_id,
        plan_hash: fixture.request.plan_hash,
        expected_reserved_component_count: 0,
        expected_claimed_component_count: 0,
        expected_installed_component_count: 0,
        expected_registry_committed_component_count: 0,
    };
    let member = RootComponentProvisioningOps::next_member_reservation(&accepted)
        .expect("next member reservation");
    let allocation = RootComponentAllocationView {
        operation_id: member.member_operation_id,
        allocation_sequence: 1,
        component: ComponentInstanceId::from_generated_bytes([31; 32]),
        component_spec: member.component_spec.clone(),
        spec_hash: member.spec_hash,
        role: CanisterRole::new("alpha"),
        provisioning_origin: ComponentProvisioningOrigin::ComponentGroup {
            operation_id: fixture.request.operation_id,
            plan_hash: fixture.request.plan_hash,
            group_placement: member.group_placement,
            member_path: member.member_path,
        },
        release_set: fixture.request.batch.active_release_set,
        progress: RootComponentAllocationProgressView::Reserved,
    };
    let advanced = RootComponentProvisioningOps::mark_member_reserved(request, &allocation)
        .expect("commit member reservation");
    (request, allocation, advanced)
}

fn commit_single_registry_member(
    member: &RootComponentProvisioningMemberView,
    installed_allocation: RootComponentAllocationView,
    installed: &RootComponentProvisioningView,
    install_request: RootComponentProvisioningAdvanceRequest,
) -> (RootComponentProvisioningView, ProvisionedMemberEvidence) {
    let registry_request = RootComponentProvisioningAdvanceRequest {
        expected_installed_component_count: 1,
        expected_registry_committed_component_count: 0,
        ..install_request
    };
    assert_eq!(
        RootComponentProvisioningOps::advance_disposition(registry_request, installed)
            .expect("Registry disposition"),
        RootComponentProvisioningAdvanceDisposition::Advance
    );
    let registry_member = RootComponentProvisioningOps::next_member_registry_commit(installed)
        .expect("next Registry member");
    assert_eq!(&registry_member, member);
    let (committed_allocation, partition) = committed_allocation(installed_allocation);
    let mut wrong_partition = partition.clone();
    wrong_partition.status = ComponentLifecycleStatus::Active;
    assert!(
        RootComponentProvisioningOps::mark_member_registry_committed(
            registry_request,
            &committed_allocation,
            &wrong_partition,
        )
        .is_err()
    );
    let registered = RootComponentProvisioningOps::mark_member_registry_committed(
        registry_request,
        &committed_allocation,
        &partition,
    )
    .expect("commit member Registry progress");
    assert_eq!(
        RootComponentProvisioningOps::advance_disposition(registry_request, &registered)
            .expect("Registry response-loss replay"),
        RootComponentProvisioningAdvanceDisposition::Replay
    );
    (
        registered,
        ProvisionedMemberEvidence {
            member: member.clone(),
            allocation: committed_allocation,
            partition,
        },
    )
}

fn assert_restart_retains_registered(
    request: RootComponentProvisioningAdvanceRequest,
    registered: &RootComponentProvisioningView,
) {
    let snapshot = RootComponentProvisioningStore::export();
    RootComponentProvisioningStore::import(snapshot);
    let restored = RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
    })
    .expect("restored Registry progress");
    assert_eq!(&restored, registered);
}

fn finalize_single_provisioned_result(
    fixture: &Fixture,
    registered: &RootComponentProvisioningView,
    evidence: ProvisionedMemberEvidence,
    request: RootComponentProvisioningAdvanceRequest,
) -> RootComponentProvisioningView {
    let result =
        provisioned_result_record(registered, &[evidence]).expect("complete provisioned result");
    let provisioned = commit_provisioned_result(request, 300, result)
        .expect("commit terminal provisioned receipt");
    assert_eq!(
        provisioned.phase,
        RootComponentProvisioningPhase::Provisioned
    );
    assert_eq!(provisioned.provisioned_at_ns, Some(300));
    assert_eq!(
        provisioned
            .result
            .as_ref()
            .expect("provisioned result")
            .placements
            .len(),
        1
    );
    assert_ne!(
        provisioned.receipt_content_hash,
        registered.receipt_content_hash
    );
    assert_eq!(
        RootComponentProvisioningOps::advance_disposition(request, &provisioned)
            .expect("terminal replay disposition"),
        RootComponentProvisioningAdvanceDisposition::Complete
    );
    assert_eq!(
        RootComponentProvisioningOps::acceptance_replay(&fixture.request)
            .expect("terminal acceptance replay")
            .expect("terminal receipt"),
        provisioned
    );
    provisioned
}

fn single_provisioned_fixture() -> (Fixture, RootComponentProvisioningView) {
    let fixture = fixture();
    let (reservation_request, allocation, reserved) = reserve_single_member(&fixture);
    let member =
        RootComponentProvisioningOps::next_member_claim(&reserved).expect("single reserved member");
    let claim_request = RootComponentProvisioningAdvanceRequest {
        expected_reserved_component_count: 1,
        ..reservation_request
    };
    let claimed_allocation = claimed_allocation(
        allocation,
        principal(31),
        fixture.request.batch.root.fleet_subnet_root,
    );
    let _claimed =
        RootComponentProvisioningOps::mark_member_claimed(claim_request, &claimed_allocation)
            .expect("claim single member");
    let install_request = RootComponentProvisioningAdvanceRequest {
        expected_claimed_component_count: 1,
        ..claim_request
    };
    let installed_allocation =
        installed_allocation(claimed_allocation, &fixture.request.batch.root);
    let installed =
        RootComponentProvisioningOps::mark_member_installed(install_request, &installed_allocation)
            .expect("install single member");
    let (registered, evidence) =
        commit_single_registry_member(&member, installed_allocation, &installed, install_request);
    let complete = RootComponentProvisioningAdvanceRequest {
        expected_installed_component_count: 1,
        expected_registry_committed_component_count: 1,
        ..install_request
    };
    let provisioned = finalize_single_provisioned_result(&fixture, &registered, evidence, complete);
    (fixture, provisioned)
}

fn publication_directory(
    fixture: &Fixture,
    version: FleetRegistryVersion,
) -> FleetDirectorySnapshot {
    FleetDirectorySnapshot {
        provenance: FleetDirectoryProvenance {
            registry: version,
            source_fleet_subnet_root: fixture.request.batch.root.fleet_subnet_root,
        },
        fleet_subnet_roots: vec![FleetSubnetRootDirectoryEntry {
            placement_subnet: fixture.request.batch.root.placement_subnet,
            fleet_subnet_root: fixture.request.batch.root.fleet_subnet_root,
            status: FleetSubnetRootStatus::Active,
        }],
        services: vec![],
    }
}

fn assert_terminal_provisioning_corruption_rejects(fixture: &Fixture) {
    let exact = RootComponentProvisioningStore::export();
    let mut corrupted = exact.clone();
    let RootComponentProvisioningStateRecordPhase::Provisioned {
        receipt_content_hash,
        ..
    } = &mut corrupted.operations[0].state
    else {
        panic!("terminal fixture must retain Provisioned state")
    };
    *receipt_content_hash = [99; 32];
    RootComponentProvisioningStore::import(corrupted);
    assert!(
        RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
            operation_id: fixture.request.operation_id,
            plan_hash: fixture.request.plan_hash,
        })
        .is_err()
    );

    let mut corrupted = exact.clone();
    let RootComponentProvisioningStateRecordPhase::Provisioned { result, .. } =
        &mut corrupted.operations[0].state
    else {
        panic!("terminal fixture must retain Provisioned state")
    };
    result.placements[0].members[0].component_registry_content_hash = [0; 32];
    RootComponentProvisioningStore::import(corrupted);
    assert!(
        RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
            operation_id: fixture.request.operation_id,
            plan_hash: fixture.request.plan_hash,
        })
        .is_err()
    );
    RootComponentProvisioningStore::import(exact);
}

#[test]
fn exact_acceptance_replays_across_restart_without_mutating_capacity() {
    let fixture = fixture();
    assert!(RootComponentProvisioningOps::require_ordinary_allocation_open().is_ok());
    assert!(RootComponentProvisioningOps::require_root_draining_open().is_ok());
    let accepted =
        RootComponentProvisioningOps::accept(fixture.request.clone(), &fixture.validation, 100)
            .expect("accept batch");
    let replay = RootComponentProvisioningOps::acceptance_replay(&fixture.request)
        .expect("replay lookup")
        .expect("accepted replay");
    assert_eq!(replay, accepted);
    assert_eq!(
        RootComponentProvisioningOps::tracked_group_placements().expect("placement count"),
        1
    );
    assert!(RootComponentProvisioningOps::require_ordinary_allocation_open().is_err());
    assert!(RootComponentProvisioningOps::require_root_draining_open().is_err());

    let snapshot = RootComponentProvisioningStore::export();
    RootComponentProvisioningStore::import(snapshot.clone());
    assert_eq!(RootComponentProvisioningStore::export(), snapshot);
    assert_eq!(
        RootComponentProvisioningOps::accept(fixture.request, &fixture.validation, 999,)
            .expect("exact acceptance retry"),
        accepted
    );
    assert_eq!(
        RootComponentProvisioningOps::tracked_group_placements().expect("placement count"),
        1
    );
}

#[test]
fn acceptance_persists_maximum_encoded_operation_and_placement_authority() {
    let mut fixture = fixture();
    fixture.request.operation_id = [u8::MAX; 32];
    fixture.request.plan_hash = [u8::MAX; 32];

    let accepted =
        RootComponentProvisioningOps::accept(fixture.request.clone(), &fixture.validation, 100)
            .expect("accept maximum encoded placement authority");

    assert_eq!(accepted.operation_id, fixture.request.operation_id);
    assert_eq!(
        RootComponentProvisioningOps::acceptance_replay(&fixture.request)
            .expect("replay maximum encoded placement authority")
            .expect("maximum encoded placement authority receipt"),
        accepted
    );
}

#[test]
fn reservation_advance_is_cursor_bound_and_response_loss_safe() {
    let fixture = fixture();
    let accepted =
        RootComponentProvisioningOps::accept(fixture.request.clone(), &fixture.validation, 100)
            .expect("accept batch");
    let request = RootComponentProvisioningAdvanceRequest {
        operation_id: fixture.request.operation_id,
        plan_hash: fixture.request.plan_hash,
        expected_reserved_component_count: 0,
        expected_claimed_component_count: 0,
        expected_installed_component_count: 0,
        expected_registry_committed_component_count: 0,
    };
    assert_eq!(
        RootComponentProvisioningOps::advance_disposition(request, &accepted)
            .expect("advance disposition"),
        RootComponentProvisioningAdvanceDisposition::Advance
    );
    let member = RootComponentProvisioningOps::next_member_reservation(&accepted)
        .expect("next member reservation");
    assert_eq!(
        member.member_operation_id,
        [
            171, 220, 109, 38, 164, 5, 152, 44, 236, 134, 217, 253, 36, 55, 213, 121, 56, 239, 238,
            30, 99, 196, 36, 234, 206, 2, 149, 236, 72, 124, 86, 131,
        ]
    );
    let (_request, _allocation, advanced) = reserve_single_member(&fixture);
    assert_eq!(advanced.reservation_cursor.reserved_component_count, 1);
    assert_eq!(advanced.reservation_cursor.placement_index, 1);
    assert_eq!(advanced.reservation_cursor.member_index, 0);
    assert_eq!(
        RootComponentProvisioningOps::advance_disposition(request, &advanced)
            .expect("response-loss replay"),
        RootComponentProvisioningAdvanceDisposition::Replay
    );
}

#[test]
fn claim_install_and_registry_advances_are_context_bound_restart_safe_and_terminal() {
    let fixture = fixture();
    let (request, allocation, advanced) = reserve_single_member(&fixture);
    let claim_request = RootComponentProvisioningAdvanceRequest {
        expected_reserved_component_count: 1,
        ..request
    };
    assert_eq!(
        RootComponentProvisioningOps::advance_disposition(claim_request, &advanced)
            .expect("claim disposition"),
        RootComponentProvisioningAdvanceDisposition::Advance
    );

    let canister = principal(31);
    let claimed_allocation = claimed_allocation(
        allocation,
        canister,
        fixture.request.batch.root.fleet_subnet_root,
    );
    let member =
        RootComponentProvisioningOps::next_member_claim(&advanced).expect("next claimed member");
    let context = RootComponentProvisioningOps::member_deployment_context(
        &advanced,
        &member,
        &claimed_allocation,
    )
    .expect("plan-derived deployment context");
    assert_group_context(context, canister, fixture.request.configuration_digest);

    let claimed =
        RootComponentProvisioningOps::mark_member_claimed(claim_request, &claimed_allocation)
            .expect("commit member claim");
    assert_eq!(claimed.claim_cursor.claimed_component_count, 1);
    assert_eq!(claimed.claim_cursor.placement_index, 1);
    assert_eq!(claimed.claim_cursor.member_index, 0);
    let response = status_response(claimed.clone());
    assert_eq!(response.reserved_component_count, 1);
    assert_eq!(response.claimed_component_count, 1);
    assert_eq!(response.installed_component_count, 0);
    assert_eq!(response.registry_committed_component_count, 0);
    assert_eq!(
        RootComponentProvisioningOps::advance_disposition(claim_request, &claimed)
            .expect("claim response-loss replay"),
        RootComponentProvisioningAdvanceDisposition::Replay
    );
    let install_request = RootComponentProvisioningAdvanceRequest {
        expected_claimed_component_count: 1,
        ..claim_request
    };
    assert_eq!(
        RootComponentProvisioningOps::advance_disposition(install_request, &claimed)
            .expect("install disposition"),
        RootComponentProvisioningAdvanceDisposition::Advance
    );
    assert!(
        RootComponentProvisioningOps::mark_member_installed(install_request, &claimed_allocation)
            .is_err(),
        "aggregate install progress requires an independently verified runtime"
    );
    let install_member =
        RootComponentProvisioningOps::next_member_install(&claimed).expect("next install member");
    assert_eq!(install_member, member);
    let installed_allocation =
        installed_allocation(claimed_allocation, &fixture.request.batch.root);
    let installed =
        RootComponentProvisioningOps::mark_member_installed(install_request, &installed_allocation)
            .expect("commit member install");
    assert_eq!(installed.install_cursor.installed_component_count, 1);
    assert_eq!(installed.install_cursor.placement_index, 1);
    assert_eq!(installed.install_cursor.member_index, 0);
    assert_eq!(
        RootComponentProvisioningOps::advance_disposition(install_request, &installed)
            .expect("install response-loss replay"),
        RootComponentProvisioningAdvanceDisposition::Replay
    );
    let contradictory = RootComponentProvisioningAdvanceRequest {
        expected_reserved_component_count: 0,
        ..install_request
    };
    assert!(RootComponentProvisioningOps::advance_disposition(contradictory, &installed).is_err());
    let (registered, evidence) =
        commit_single_registry_member(&member, installed_allocation, &installed, install_request);
    assert_eq!(
        registered
            .registry_cursor
            .registry_committed_component_count,
        1
    );
    assert_eq!(registered.registry_cursor.placement_index, 1);
    assert_eq!(registered.registry_cursor.member_index, 0);
    let complete = RootComponentProvisioningAdvanceRequest {
        expected_registry_committed_component_count: 1,
        expected_installed_component_count: 1,
        ..install_request
    };
    assert_eq!(
        RootComponentProvisioningOps::advance_disposition(complete, &registered)
            .expect("terminal disposition"),
        RootComponentProvisioningAdvanceDisposition::Advance
    );

    let provisioned = finalize_single_provisioned_result(&fixture, &registered, evidence, complete);

    assert_restart_retains_registered(request, &provisioned);
    assert_terminal_provisioning_corruption_rejects(&fixture);
}

#[test]
fn directory_publication_journals_each_delivery_and_replays_terminal_receipt() {
    let (fixture, published, directory, final_request) = single_published_fixture();
    assert_eq!(published.phase, RootComponentProvisioningPhase::Published);
    assert_eq!(published.published_at_ns, Some(500));
    assert_eq!(published.published_component_count, 1);
    assert_eq!(
        published
            .publication
            .as_ref()
            .expect("publication evidence")
            .component_directories
            .len(),
        1
    );
    assert_eq!(
        RootComponentProvisioningOps::begin_publication(&final_request, &directory, 999)
            .expect("replay terminal publication"),
        published
    );
    let mut conflicting = final_request;
    conflicting.published_fleet_registry.content_hash[0] ^= 1;
    assert!(
        RootComponentProvisioningOps::begin_publication(&conflicting, &directory, 999).is_err()
    );
    assert_eq!(fixture.request.operation_id, published.operation_id);
}

#[test]
fn runtime_activation_begins_only_after_publication_and_replays_exactly() {
    let (_fixture, published, _directory, _publication_request) = single_published_fixture();
    let request = RootComponentActivationRequest {
        operation_id: published.operation_id,
        plan_hash: published.plan_hash,
        expected_activated_component_count: 0,
        expected_root_runtime_active: false,
    };
    let activating = RootComponentProvisioningOps::begin_activation(&request, 600)
        .expect("begin runtime activation");
    assert_eq!(activating.phase, RootComponentProvisioningPhase::Published);
    assert_eq!(activating.activation_started_at_ns, Some(600));
    assert_eq!(activating.activated_component_count, 0);
    assert!(!activating.root_runtime_active);
    assert!(
        RootComponentProvisioningOps::next_activation_member(&activating)
            .expect("select activation member")
            .is_some()
    );
    assert!(
        RootComponentProvisioningOps::finalize_runtimes_active(
            &request,
            RootComponentActivationEvidence {
                fleet_activation_operation_id: [61; 32],
                initial_inventory_hash: [62; 32],
                component_count: 1,
                root_activated_at_ns: 700,
            },
            700,
        )
        .is_err()
    );
    assert_eq!(
        RootComponentProvisioningOps::begin_activation(&request, 999)
            .expect("replay runtime activation start"),
        activating
    );
    let conflicting = RootComponentActivationRequest {
        expected_root_runtime_active: true,
        ..request
    };
    assert!(RootComponentProvisioningOps::begin_activation(&conflicting, 999).is_err());
}

#[test]
fn terminal_batch_releases_only_the_active_fence_for_the_next_operation() {
    let mut fixture = fixture();
    let mut next_request = fixture.request.clone();
    let next_validation = fixture.validation.clone();
    next_request.operation_id = [71; 32];
    next_request.plan_hash = [72; 32];
    next_request.batch.placements[0].group_placement.ordinal = 1;
    fixture.request.batch.placements.clear();
    fixture.validation = RootComponentProvisioningBatchValidation {
        placement_count: 0,
        component_count: 0,
        component_spec_counts: BTreeMap::new(),
        component_roles: BTreeSet::new(),
    };
    let accepted =
        RootComponentProvisioningOps::accept(fixture.request.clone(), &fixture.validation, 100)
            .expect("accept empty root batch");
    let advance = RootComponentProvisioningAdvanceRequest {
        operation_id: accepted.operation_id,
        plan_hash: accepted.plan_hash,
        expected_reserved_component_count: 0,
        expected_claimed_component_count: 0,
        expected_installed_component_count: 0,
        expected_registry_committed_component_count: 0,
    };
    let provisioned = RootComponentProvisioningOps::finalize_provisioned(advance, 200)
        .expect("finalize empty root batch");

    let published_registry = FleetRegistryVersion {
        authority: provisioned.fleet_registry.authority.clone(),
        revision: provisioned.fleet_registry.revision + 1,
        content_hash: [44; 32],
    };
    let directory = publication_directory(&fixture, published_registry.clone());
    let publication = RootComponentPublicationRequest {
        operation_id: provisioned.operation_id,
        plan_hash: provisioned.plan_hash,
        published_fleet_registry: published_registry,
        expected_published_component_count: 0,
    };
    let publishing = RootComponentProvisioningOps::begin_publication(&publication, &directory, 300)
        .expect("begin empty root publication");
    assert!(
        RootComponentProvisioningOps::next_publication_member(&publishing)
            .expect("inspect empty root publication")
            .is_none()
    );
    let published = RootComponentProvisioningOps::finalize_published(&publication, 400)
        .expect("finalize empty root publication");

    let activation = RootComponentActivationRequest {
        operation_id: published.operation_id,
        plan_hash: published.plan_hash,
        expected_activated_component_count: 0,
        expected_root_runtime_active: false,
    };
    let activating = RootComponentProvisioningOps::begin_activation(&activation, 500)
        .expect("begin empty root activation");
    assert!(
        RootComponentProvisioningOps::next_activation_member(&activating)
            .expect("inspect empty root activation")
            .is_none()
    );
    let active = RootComponentProvisioningOps::finalize_runtimes_active(
        &activation,
        RootComponentActivationEvidence {
            fleet_activation_operation_id: [61; 32],
            initial_inventory_hash: [62; 32],
            component_count: 0,
            root_activated_at_ns: 600,
        },
        600,
    )
    .expect("finalize empty root activation");

    assert_eq!(active.phase, RootComponentProvisioningPhase::RuntimesActive);
    assert_eq!(active.component_count, 0);
    assert!(active.root_runtime_active);
    assert_eq!(
        RootComponentProvisioningStore::state(),
        crate::storage::stable::component_provisioning::RootComponentProvisioningStateRecord {
            tracked_group_placements: 0,
            active_operation_id: None,
        }
    );

    reserve_claim_and_install_later_operation_member(next_request, &next_validation);
    assert_eq!(
        RootComponentProvisioningOps::status(RootComponentProvisioningStatusRequest {
            operation_id: active.operation_id,
            plan_hash: active.plan_hash,
        })
        .expect("terminal historical receipt remains replayable"),
        active
    );
    assert_eq!(
        RootComponentProvisioningStore::state(),
        crate::storage::stable::component_provisioning::RootComponentProvisioningStateRecord {
            tracked_group_placements: 1,
            active_operation_id: Some([71; 32]),
        }
    );
}

fn reserve_claim_and_install_later_operation_member(
    request: RootComponentProvisioningAcceptanceRequest,
    validation: &RootComponentProvisioningBatchValidation,
) {
    let accepted = RootComponentProvisioningOps::accept(request, validation, 700)
        .expect("accept later scale-out-shaped batch");
    assert_eq!(accepted.phase, RootComponentProvisioningPhase::Accepted);
    assert_eq!(accepted.batch.placements[0].group_placement.ordinal, 1);
    let reservation_request = RootComponentProvisioningAdvanceRequest {
        operation_id: accepted.operation_id,
        plan_hash: accepted.plan_hash,
        expected_reserved_component_count: 0,
        expected_claimed_component_count: 0,
        expected_installed_component_count: 0,
        expected_registry_committed_component_count: 0,
    };
    let member = RootComponentProvisioningOps::next_member_reservation(&accepted)
        .expect("next scale-out member reservation");
    let allocation = RootComponentAllocationView {
        operation_id: member.member_operation_id,
        allocation_sequence: 2,
        component: ComponentInstanceId::from_generated_bytes([73; 32]),
        component_spec: member.component_spec.clone(),
        spec_hash: member.spec_hash,
        role: CanisterRole::new("alpha"),
        provisioning_origin: ComponentProvisioningOrigin::ComponentGroup {
            operation_id: accepted.operation_id,
            plan_hash: accepted.plan_hash,
            group_placement: member.group_placement,
            member_path: member.member_path,
        },
        release_set: accepted.batch.active_release_set,
        progress: RootComponentAllocationProgressView::Reserved,
    };
    let reserved =
        RootComponentProvisioningOps::mark_member_reserved(reservation_request, &allocation)
            .expect("reserve scale-out member identity");
    assert_eq!(reserved.reservation_cursor.reserved_component_count, 1);
    assert_eq!(reserved.claim_cursor.claimed_component_count, 0);
    assert_eq!(
        RootComponentProvisioningOps::advance_disposition(reservation_request, &reserved)
            .expect("lost reservation response replays"),
        RootComponentProvisioningAdvanceDisposition::Replay
    );

    let claim_request = RootComponentProvisioningAdvanceRequest {
        expected_reserved_component_count: 1,
        ..reservation_request
    };
    let claimed_allocation = claimed_allocation(
        allocation,
        principal(74),
        accepted.batch.root.fleet_subnet_root,
    );
    let claimed =
        RootComponentProvisioningOps::mark_member_claimed(claim_request, &claimed_allocation)
            .expect("claim scale-out prepaid Canister");
    assert_eq!(claimed.claim_cursor.claimed_component_count, 1);
    assert_eq!(claimed.install_cursor.installed_component_count, 0);
    assert_eq!(
        RootComponentProvisioningOps::advance_disposition(claim_request, &claimed)
            .expect("lost claim response replays"),
        RootComponentProvisioningAdvanceDisposition::Replay
    );

    let install_request = RootComponentProvisioningAdvanceRequest {
        expected_claimed_component_count: 1,
        ..claim_request
    };
    let installed_allocation = installed_allocation(claimed_allocation, &accepted.batch.root);
    let installed =
        RootComponentProvisioningOps::mark_member_installed(install_request, &installed_allocation)
            .expect("install scale-out Component");
    assert_eq!(installed.install_cursor.installed_component_count, 1);
    assert_eq!(
        installed.registry_cursor.registry_committed_component_count,
        0
    );
    assert_eq!(
        RootComponentProvisioningOps::advance_disposition(install_request, &installed)
            .expect("lost install response replays"),
        RootComponentProvisioningAdvanceDisposition::Replay
    );
}

fn single_published_fixture() -> (
    Fixture,
    RootComponentProvisioningView,
    FleetDirectorySnapshot,
    RootComponentPublicationRequest,
) {
    let (fixture, provisioned) = single_provisioned_fixture();
    let published_registry = FleetRegistryVersion {
        authority: provisioned.fleet_registry.authority.clone(),
        revision: provisioned.fleet_registry.revision + 1,
        content_hash: [44; 32],
    };
    let directory = publication_directory(&fixture, published_registry.clone());
    let first_request = RootComponentPublicationRequest {
        operation_id: provisioned.operation_id,
        plan_hash: provisioned.plan_hash,
        published_fleet_registry: published_registry,
        expected_published_component_count: 0,
    };
    let delivered = record_one_publication_delivery(&first_request, &directory);
    assert_eq!(delivered.published_component_count, 1);
    assert_eq!(
        delivered
            .publication
            .as_ref()
            .expect("publication evidence")
            .component_directories
            .len(),
        1
    );
    assert_eq!(
        RootComponentProvisioningOps::begin_publication(&first_request, &directory, 999)
            .expect("replay lost delivery response"),
        delivered
    );
    let final_request = RootComponentPublicationRequest {
        expected_published_component_count: 1,
        ..first_request
    };
    let published = RootComponentProvisioningOps::finalize_published(&final_request, 500)
        .expect("freeze terminal publication receipt");
    assert_ne!(
        published.receipt_content_hash,
        provisioned.receipt_content_hash
    );
    (fixture, published, directory, final_request)
}

fn record_one_publication_delivery(
    request: &RootComponentPublicationRequest,
    directory: &FleetDirectorySnapshot,
) -> RootComponentProvisioningView {
    let publishing = RootComponentProvisioningOps::begin_publication(request, directory, 400)
        .expect("begin exact Directory publication");
    assert_eq!(publishing.published_component_count, 0);
    assert_eq!(
        publishing
            .publication
            .as_ref()
            .expect("publication evidence")
            .component_group_directories
            .len(),
        1
    );
    let member = RootComponentProvisioningOps::next_publication_member(&publishing)
        .expect("select publication member")
        .expect("one publication member");
    let authority_hash = [55; 32];
    let intent = RootComponentProvisioningOps::begin_publication_delivery(
        request,
        &member,
        authority_hash,
        401,
    )
    .expect("persist Directory delivery intent");
    assert_eq!(
        intent
            .publication_in_flight
            .as_ref()
            .expect("durable publication intent")
            .canister_id,
        member.binding.canister_id
    );
    let snapshot = RootComponentProvisioningStore::export();
    RootComponentProvisioningStore::import(snapshot);
    assert_eq!(
        RootComponentProvisioningOps::begin_publication_delivery(
            request,
            &member,
            authority_hash,
            999,
        )
        .expect("reconcile response loss"),
        intent
    );
    assert!(
        RootComponentProvisioningOps::begin_publication_delivery(request, &member, [56; 32], 402,)
            .is_err()
    );
    RootComponentProvisioningOps::record_publication_delivery(request, &member, authority_hash)
        .expect("commit independently observed Directory delivery")
}

#[test]
fn prepaid_claim_cannot_precede_complete_identity_reservation() {
    let fixture = fixture();
    let accepted =
        RootComponentProvisioningOps::accept(fixture.request.clone(), &fixture.validation, 100)
            .expect("accept batch");
    let member = RootComponentProvisioningOps::next_member_reservation(&accepted)
        .expect("unreserved member");
    let allocation = RootComponentAllocationView {
        operation_id: member.member_operation_id,
        allocation_sequence: 1,
        component: ComponentInstanceId::from_generated_bytes([31; 32]),
        component_spec: member.component_spec.clone(),
        spec_hash: member.spec_hash,
        role: CanisterRole::new("alpha"),
        provisioning_origin: ComponentProvisioningOrigin::ComponentGroup {
            operation_id: fixture.request.operation_id,
            plan_hash: fixture.request.plan_hash,
            group_placement: member.group_placement.clone(),
            member_path: member.member_path,
        },
        release_set: fixture.request.batch.active_release_set,
        progress: RootComponentAllocationProgressView::Reserved,
    };
    let allocation = claimed_allocation(
        allocation,
        principal(31),
        fixture.request.batch.root.fleet_subnet_root,
    );
    let request = RootComponentProvisioningAdvanceRequest {
        operation_id: fixture.request.operation_id,
        plan_hash: fixture.request.plan_hash,
        expected_reserved_component_count: 0,
        expected_claimed_component_count: 0,
        expected_installed_component_count: 0,
        expected_registry_committed_component_count: 0,
    };
    assert!(RootComponentProvisioningOps::next_member_claim(&accepted).is_err());
    assert!(RootComponentProvisioningOps::mark_member_claimed(request, &allocation).is_err());
    let installed = installed_allocation(allocation, &fixture.request.batch.root);
    assert!(RootComponentProvisioningOps::next_member_install(&accepted).is_err());
    assert!(RootComponentProvisioningOps::mark_member_installed(request, &installed).is_err());
    let (committed, partition) = committed_allocation(installed);
    assert!(RootComponentProvisioningOps::next_member_registry_commit(&accepted).is_err());
    assert!(
        RootComponentProvisioningOps::mark_member_registry_committed(
            request, &committed, &partition,
        )
        .is_err()
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one canonical four-phase journey makes ordering and identity reuse explicit"
)]
fn reservation_cursor_crosses_canonical_placements_without_reusing_identity() {
    let mut fixture = fixture();
    let mut second_placement = fixture.request.batch.placements[0].clone();
    second_placement.group_placement.ordinal = 1;
    fixture.request.batch.placements.push(second_placement);
    fixture.validation.placement_count = 2;
    fixture.validation.component_count = 2;
    *fixture
        .validation
        .component_spec_counts
        .values_mut()
        .next()
        .expect("Component Spec count") = 2;

    let mut current =
        RootComponentProvisioningOps::accept(fixture.request.clone(), &fixture.validation, 100)
            .expect("accept two-placement batch");
    let mut operation_ids = BTreeSet::new();
    let mut allocations = Vec::new();
    let mut claimed_allocations = Vec::new();
    let mut installed_allocations = Vec::new();
    let mut registry_evidence = Vec::new();
    for expected in 0..2 {
        let request = RootComponentProvisioningAdvanceRequest {
            operation_id: fixture.request.operation_id,
            plan_hash: fixture.request.plan_hash,
            expected_reserved_component_count: expected,
            expected_claimed_component_count: 0,
            expected_installed_component_count: 0,
            expected_registry_committed_component_count: 0,
        };
        let member = RootComponentProvisioningOps::next_member_reservation(&current)
            .expect("next placement member");
        assert_eq!(member.group_placement.ordinal, expected);
        assert!(operation_ids.insert(member.member_operation_id));
        let allocation = RootComponentAllocationView {
            operation_id: member.member_operation_id,
            allocation_sequence: u64::from(expected) + 1,
            component: ComponentInstanceId::from_generated_bytes(
                [u8::try_from(expected).expect("small ordinal") + 40; 32],
            ),
            component_spec: member.component_spec.clone(),
            spec_hash: member.spec_hash,
            role: CanisterRole::new("alpha"),
            provisioning_origin: ComponentProvisioningOrigin::ComponentGroup {
                operation_id: fixture.request.operation_id,
                plan_hash: fixture.request.plan_hash,
                group_placement: member.group_placement,
                member_path: member.member_path,
            },
            release_set: fixture.request.batch.active_release_set,
            progress: RootComponentAllocationProgressView::Reserved,
        };
        current = RootComponentProvisioningOps::mark_member_reserved(request, &allocation)
            .expect("advance placement member");
        allocations.push(allocation);
    }
    assert_eq!(current.reservation_cursor.reserved_component_count, 2);
    assert_eq!(current.reservation_cursor.placement_index, 2);
    assert_eq!(current.reservation_cursor.member_index, 0);

    for expected in 0..2 {
        let request = RootComponentProvisioningAdvanceRequest {
            operation_id: fixture.request.operation_id,
            plan_hash: fixture.request.plan_hash,
            expected_reserved_component_count: 2,
            expected_claimed_component_count: expected,
            expected_installed_component_count: 0,
            expected_registry_committed_component_count: 0,
        };
        let member = RootComponentProvisioningOps::next_member_claim(&current)
            .expect("next canonical claim member");
        assert_eq!(member.group_placement.ordinal, expected);
        let allocation = allocations
            .iter()
            .find(|allocation| allocation.operation_id == member.member_operation_id)
            .expect("reserved claim member")
            .clone();
        let allocation = claimed_allocation(
            allocation,
            principal(u8::try_from(expected).expect("small ordinal") + 50),
            fixture.request.batch.root.fleet_subnet_root,
        );
        current = RootComponentProvisioningOps::mark_member_claimed(request, &allocation)
            .expect("advance placement claim");
        claimed_allocations.push(allocation);
    }
    assert_eq!(current.claim_cursor.claimed_component_count, 2);
    assert_eq!(current.claim_cursor.placement_index, 2);
    assert_eq!(current.claim_cursor.member_index, 0);

    for expected in 0..2 {
        let request = RootComponentProvisioningAdvanceRequest {
            operation_id: fixture.request.operation_id,
            plan_hash: fixture.request.plan_hash,
            expected_reserved_component_count: 2,
            expected_claimed_component_count: 2,
            expected_installed_component_count: expected,
            expected_registry_committed_component_count: 0,
        };
        let member = RootComponentProvisioningOps::next_member_install(&current)
            .expect("next canonical install member");
        assert_eq!(member.group_placement.ordinal, expected);
        let allocation = claimed_allocations
            .iter()
            .find(|allocation| allocation.operation_id == member.member_operation_id)
            .expect("claimed install member")
            .clone();
        let allocation = installed_allocation(allocation, &fixture.request.batch.root);
        current = RootComponentProvisioningOps::mark_member_installed(request, &allocation)
            .expect("advance placement install");
        installed_allocations.push(allocation);
    }
    assert_eq!(current.install_cursor.installed_component_count, 2);
    assert_eq!(current.install_cursor.placement_index, 2);
    assert_eq!(current.install_cursor.member_index, 0);

    for expected in 0..2 {
        let request = RootComponentProvisioningAdvanceRequest {
            operation_id: fixture.request.operation_id,
            plan_hash: fixture.request.plan_hash,
            expected_reserved_component_count: 2,
            expected_claimed_component_count: 2,
            expected_installed_component_count: 2,
            expected_registry_committed_component_count: expected,
        };
        let member = RootComponentProvisioningOps::next_member_registry_commit(&current)
            .expect("next canonical Registry member");
        assert_eq!(member.group_placement.ordinal, expected);
        let allocation = installed_allocations
            .iter()
            .find(|allocation| allocation.operation_id == member.member_operation_id)
            .expect("installed Registry member")
            .clone();
        let (allocation, partition) = committed_allocation(allocation);
        current = RootComponentProvisioningOps::mark_member_registry_committed(
            request,
            &allocation,
            &partition,
        )
        .expect("advance placement Registry commitment");
        registry_evidence.push(ProvisionedMemberEvidence {
            member,
            allocation,
            partition,
        });
    }
    assert_eq!(
        current.registry_cursor.registry_committed_component_count,
        2
    );
    assert_eq!(current.registry_cursor.placement_index, 2);
    assert_eq!(current.registry_cursor.member_index, 0);
    registry_evidence.reverse();
    assert!(provisioned_result_record(&current, &registry_evidence).is_err());
    registry_evidence.reverse();
    let result = provisioned_result_record(&current, &registry_evidence)
        .expect("canonical multi-placement result");
    assert_eq!(result.placements.len(), 2);
    let complete = RootComponentProvisioningAdvanceRequest {
        operation_id: fixture.request.operation_id,
        plan_hash: fixture.request.plan_hash,
        expected_reserved_component_count: 2,
        expected_claimed_component_count: 2,
        expected_installed_component_count: 2,
        expected_registry_committed_component_count: 2,
    };
    let provisioned =
        commit_provisioned_result(complete, 300, result).expect("multi-placement terminal result");
    assert_eq!(
        provisioned.phase,
        RootComponentProvisioningPhase::Provisioned
    );
    assert_eq!(
        provisioned
            .result
            .expect("multi-placement result")
            .placements
            .len(),
        2
    );
}

#[test]
fn conflicting_operation_and_active_batch_reject_without_new_reservations() {
    let fixture = fixture();
    RootComponentProvisioningOps::accept(fixture.request.clone(), &fixture.validation, 100)
        .expect("accept batch");

    let mut conflicting = fixture.request.clone();
    conflicting.plan_hash = [13; 32];
    assert!(RootComponentProvisioningOps::acceptance_replay(&conflicting).is_err());

    let mut second = fixture.request;
    second.operation_id = [14; 32];
    second.plan_hash = [15; 32];
    second.batch.placements[0].group_placement.ordinal = 1;
    assert!(RootComponentProvisioningOps::require_acceptance_open(second.operation_id).is_err());
    assert!(RootComponentProvisioningOps::accept(second, &fixture.validation, 101).is_err());
    assert_eq!(
        RootComponentProvisioningOps::tracked_group_placements().expect("placement count"),
        1
    );
}

#[test]
fn corrupted_receipt_index_or_aggregate_state_fails_closed() {
    let fixture = fixture();
    RootComponentProvisioningOps::accept(fixture.request.clone(), &fixture.validation, 100)
        .expect("accept batch");
    let exact = RootComponentProvisioningStore::export();

    let mut corrupted = exact.clone();
    let state = &mut corrupted.operations[0].state;
    let RootComponentProvisioningStateRecordPhase::Accepted {
        receipt_content_hash,
        ..
    } = state
    else {
        panic!("fresh fixture must retain Accepted state")
    };
    *receipt_content_hash = [99; 32];
    RootComponentProvisioningStore::import(corrupted);
    assert!(RootComponentProvisioningOps::acceptance_replay(&fixture.request).is_err());

    let mut corrupted = exact.clone();
    corrupted.placements[0].1.plan_hash = [98; 32];
    RootComponentProvisioningStore::import(corrupted);
    assert!(RootComponentProvisioningOps::acceptance_replay(&fixture.request).is_err());

    let mut corrupted = exact.clone();
    corrupted.state.tracked_group_placements = 2;
    RootComponentProvisioningStore::import(corrupted);
    assert!(RootComponentProvisioningOps::acceptance_replay(&fixture.request).is_err());

    let mut corrupted = exact.clone();
    let RootComponentProvisioningStateRecordPhase::Accepted {
        reservation_cursor, ..
    } = &mut corrupted.operations[0].state
    else {
        panic!("fresh fixture must retain Accepted state")
    };
    reservation_cursor.content_hash = [97; 32];
    RootComponentProvisioningStore::import(corrupted);
    assert!(RootComponentProvisioningOps::acceptance_replay(&fixture.request).is_err());

    let mut corrupted = exact.clone();
    let RootComponentProvisioningStateRecordPhase::Accepted { claim_cursor, .. } =
        &mut corrupted.operations[0].state
    else {
        panic!("fresh fixture must retain Accepted state")
    };
    claim_cursor.content_hash = [96; 32];
    RootComponentProvisioningStore::import(corrupted);
    assert!(RootComponentProvisioningOps::acceptance_replay(&fixture.request).is_err());

    let mut corrupted = exact.clone();
    let RootComponentProvisioningStateRecordPhase::Accepted { install_cursor, .. } =
        &mut corrupted.operations[0].state
    else {
        panic!("fresh fixture must retain Accepted state")
    };
    install_cursor.content_hash = [95; 32];
    RootComponentProvisioningStore::import(corrupted);
    assert!(RootComponentProvisioningOps::acceptance_replay(&fixture.request).is_err());

    let mut corrupted = exact;
    let RootComponentProvisioningStateRecordPhase::Accepted {
        registry_cursor, ..
    } = &mut corrupted.operations[0].state
    else {
        panic!("fresh fixture must retain Accepted state")
    };
    registry_cursor.content_hash = [94; 32];
    RootComponentProvisioningStore::import(corrupted);
    assert!(RootComponentProvisioningOps::acceptance_replay(&fixture.request).is_err());
}

#[test]
fn member_origin_is_valid_only_under_exact_accepted_authority() {
    let fixture = fixture();
    RootComponentProvisioningOps::accept(fixture.request.clone(), &fixture.validation, 100)
        .expect("accept batch");
    let placement = &fixture.request.batch.placements[0];
    let entry = &placement.entries[0];
    let origin = ComponentProvisioningOrigin::ComponentGroup {
        operation_id: fixture.request.operation_id,
        plan_hash: fixture.request.plan_hash,
        group_placement: placement.group_placement.clone(),
        member_path: entry.member_path.clone(),
    };
    RootComponentProvisioningOps::validate_member_provisioning_origin(
        &origin,
        &entry.component_spec,
        entry.spec_hash,
    )
    .expect("retained origin");
    let mut wrong_origin = origin;
    let ComponentProvisioningOrigin::ComponentGroup { member_path, .. } = &mut wrong_origin else {
        unreachable!("constructed group origin")
    };
    *member_path = ComponentGroupMemberPath::try_from(vec!["other".parse().expect("member ID")])
        .expect("member path");
    assert!(
        RootComponentProvisioningOps::validate_member_provisioning_origin(
            &wrong_origin,
            &entry.component_spec,
            entry.spec_hash,
        )
        .is_err()
    );
}
