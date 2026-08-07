//! Module: workflow::fleet_coordinator::tests
//!
//! Responsibility: qualify protected genesis commitment and canonical Coordinator queries.
//! Does not own: PocketIC installation or host effect-journal coverage.

use super::*;
use crate::storage::stable::fleet_coordinator::{
    FleetCoordinatorRegistryData, FleetCoordinatorRegistryStore,
};
use canic_core::{
    bootstrap::{
        compiled::{FleetServiceMemberPurpose, FleetServicePlacementPolicy},
        parse_config_model,
    },
    cdk::types::Cycles,
    control_plane_support::{
        config::ConfigModel,
        error::InternalErrorClass,
        ops::{
            component_provisioning_plan::ComponentProvisioningPlanOps,
            fleet_registry::FleetRegistryOps,
        },
    },
    dto::{
        component_provisioning::{
            ComponentGroupPlacementPlan, ComponentGroupPlanEntry,
            FleetComponentProvisioningOperation, FleetComponentProvisioningPhase,
            FleetComponentProvisioningPlan, FleetComponentProvisioningPrepareRequest,
            FleetComponentProvisioningStatusRequest, FleetComponentProvisioningStatusResponse,
            FleetSubnetRootProvisioningBatch,
        },
        error::ErrorCode,
        fleet_registry::{
            FleetRegistryActivationRequest, FleetServiceBinding, FleetServiceComponentBinding,
            FleetServiceMode, FleetSubnetRootDeletionCompletionRequest,
            FleetSubnetRootDeletionExecutionRequest, FleetSubnetRootDeletionReadinessIntentRequest,
            FleetSubnetRootDeletionReadinessRequest, FleetSubnetRootDeletionStatusRequest,
            FleetSubnetRootDrainingPublicationRequest, FleetSubnetRootEntry,
            FleetSubnetRootJoinRequest, FleetSubnetRootRemovalPublicationRequest,
            FleetSubnetRootStatus,
        },
        fleet_subnet_root::{
            FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES, FleetSubnetRootDrainingResponse,
            FleetSubnetRootFinalInventoryResponse,
        },
    },
    ids::{
        AppId, CanonicalNetworkId, ComponentDeploymentConfigurationDigest,
        ComponentGroupMemberPath, ComponentGroupPlacementId, ComponentInstanceId,
        ComponentSpecAdmission, CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding,
        FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetRootBinding, FleetSubnetRootLimits,
        FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    },
};

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

const COORDINATOR_CONFIG: &str = r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.project]
kind = "canister"
package = "project"

[component_specs.projects]
component_role = "project"
maximum_instances = 3

[component_groups.project_cell.components.project]
component_spec = "projects"
service = "projects"

[component_group_deployments.project_cells]
component_group = "project_cell"
service_purpose = "pool_member"
initial_placements = 2
maximum_placements = 2
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 2

[services.fleet.targets.projects]
role = "project"
component_spec = "projects"
mode = "active_pool"
placement.maximum_members_per_root = 1
placement.minimum_distinct_roots = 2
"#;

fn coordinator_config() -> ConfigModel {
    parse_config_model(COORDINATOR_CONFIG).expect("valid Coordinator config")
}

fn init_args(coordinator: Principal) -> FleetCoordinatorInitArgs {
    let component_topology = coordinator_config()
        .compile_component_topology()
        .expect("Component Topology");
    FleetCoordinatorInitArgs {
        configured_app: AppId::from("demo"),
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                        fleet_id: FleetId::from_generated_bytes([7; 32]),
                    },
                    app: AppId::from("demo"),
                },
                coordinator_subnet: SubnetId::from_principal(principal(2)),
                coordinator,
            },
            epoch: 1,
        },
        component_topology,
    }
}

#[test]
fn protected_init_commits_exact_genesis_and_supports_exact_retry() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let coordinator = principal(3);
    let controller = principal(4);
    let args = init_args(coordinator);

    FleetCoordinatorWorkflow::initialize(args.clone(), controller, true, coordinator)
        .expect("commit genesis");
    FleetCoordinatorWorkflow::initialize(args, controller, true, coordinator)
        .expect("repeat exact genesis");

    let registry = FleetCoordinatorWorkflow::registry().expect("Registry");
    let manifest = FleetCoordinatorWorkflow::manifest().expect("manifest");
    let version = FleetCoordinatorWorkflow::version().expect("version");

    assert_eq!(registry.revision, 1);
    assert_eq!(registry.component_specs.len(), 1);
    assert!(registry.fleet_subnet_roots.is_empty());
    assert_eq!(manifest.revision, registry.revision);
    assert_eq!(version.content_hash, manifest.content_hash);

    let unauthorized = FleetCoordinatorWorkflow::initialize(
        init_args(coordinator),
        principal(5),
        false,
        coordinator,
    )
    .expect_err("reject non-controller init");
    assert_eq!(
        unauthorized.public_error().map(|error| error.code),
        Some(ErrorCode::Forbidden)
    );

    let wrong_canister = FleetCoordinatorWorkflow::initialize(
        init_args(principal(6)),
        controller,
        true,
        coordinator,
    )
    .expect_err("reject wrong Coordinator binding");
    assert_eq!(
        wrong_canister.public_error().map(|error| error.code),
        Some(ErrorCode::InvalidInput)
    );
}

#[test]
fn root_join_compare_and_commit_retains_exact_response_receipts() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let coordinator = principal(13);
    let args = init_args(coordinator);
    let topology = args.component_topology.clone();
    FleetCoordinatorWorkflow::initialize(args, principal(14), true, coordinator)
        .expect("commit genesis");

    let genesis = FleetCoordinatorWorkflow::version().expect("genesis version");
    let first_entry = joining_entry(&topology, 7, 15, 1);
    let first_request = FleetSubnetRootJoinRequest {
        expected_registry: genesis.clone(),
        entry: first_entry.clone(),
    };
    let first =
        FleetCoordinatorWorkflow::join_root(first_request.clone()).expect("join first root");
    assert_eq!(first.entry, first_entry);
    assert_eq!(first.version.revision, 2);
    assert_eq!(
        FleetCoordinatorWorkflow::join_root(first_request.clone()).expect("exact first retry"),
        first
    );

    let second_entry = joining_entry(&topology, 5, 16, 2);
    let second_request = FleetSubnetRootJoinRequest {
        expected_registry: first.version.clone(),
        entry: second_entry.clone(),
    };
    let second = FleetCoordinatorWorkflow::join_root(second_request).expect("join second root");
    assert_eq!(second.version.revision, 3);
    assert_eq!(
        FleetCoordinatorWorkflow::join_root(first_request).expect("late exact first retry"),
        first,
        "the original response must survive later Registry revisions"
    );

    let registry = FleetCoordinatorWorkflow::registry().expect("joined Registry");
    assert_eq!(registry.revision, 3);
    assert_eq!(
        registry
            .fleet_subnet_roots
            .iter()
            .map(|entry| entry.placement_subnet)
            .collect::<Vec<_>>(),
        vec![second_entry.placement_subnet, first_entry.placement_subnet]
    );

    let active_version =
        assert_snapshot_acknowledgements(&registry, &first_entry, &second_entry, &second.version);
    assert_root_draining_publication(&first_entry, &second_entry, &active_version);

    let stale = FleetCoordinatorWorkflow::join_root(FleetSubnetRootJoinRequest {
        expected_registry: genesis,
        entry: joining_entry(&topology, 9, 17, 1),
    })
    .expect_err("a new root cannot commit against stale Registry authority");
    assert_eq!(
        stale.public_error().map(|error| error.code),
        Some(ErrorCode::Conflict)
    );

    let mut conflicting_entry = first_entry;
    conflicting_entry.limits.maximum_registry_bytes += 1;
    let conflict = FleetCoordinatorWorkflow::join_root(FleetSubnetRootJoinRequest {
        expected_registry: second.version,
        entry: conflicting_entry,
    })
    .expect_err("an existing root identity cannot change authority");
    assert_eq!(
        conflict.public_error().map(|error| error.code),
        Some(ErrorCode::Conflict)
    );

    let mut corrupted = FleetCoordinatorRegistryStore::export();
    corrupted
        .current
        .as_mut()
        .expect("Coordinator state")
        .root_join_receipts[0]
        .version
        .content_hash[0] ^= 1;
    FleetCoordinatorRegistryStore::import(corrupted);
    let invalid = crate::api::fleet_coordinator::FleetCoordinatorApi::registry()
        .expect_err("reject corrupted historical receipt");
    assert_eq!(invalid.code, ErrorCode::InvariantViolation);
}

#[test]
fn initial_service_publication_commits_registry_and_receipt_atomically() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let coordinator = principal(40);
    let (first, second, active_version) = activate_two_roots(coordinator);
    let service = FleetServiceBinding {
        service: "projects".parse().expect("service ID"),
        role: "project".parse().expect("role"),
        component_spec: "projects".parse().expect("Component Spec ID"),
        mode: FleetServiceMode::AuthorityReplica,
        placement: FleetServicePlacementPolicy {
            maximum_members_per_root: 1,
            minimum_distinct_roots: 2,
        },
        members: vec![
            service_member(
                FleetServiceMemberPurpose::Authority,
                41,
                first.fleet_subnet_root,
                51,
                "authority",
            ),
            service_member(
                FleetServiceMemberPurpose::Replica,
                42,
                second.fleet_subnet_root,
                52,
                "replica",
            ),
        ],
    };
    let operation_id = [43; 32];
    let plan_hash = [44; 32];
    let configuration_digest = ComponentDeploymentConfigurationDigest::from_bytes([45; 32]);
    let receipt_hashes = vec![[46; 32], [47; 32]];

    let published =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::commit_compiled_initial_services_for_test(
            active_version.clone(),
            operation_id,
            plan_hash,
            configuration_digest,
            receipt_hashes.clone(),
            vec![service.clone()],
        )
        .expect("commit complete initial service set");
    assert_eq!(published.revision, active_version.revision + 1);
    let durable = FleetCoordinatorRegistryStore::export();
    let current = durable.current.as_ref().expect("Coordinator state");
    assert_eq!(current.registry.services, vec![service.clone()]);
    assert_eq!(
        current
            .service_publication_receipt
            .as_ref()
            .expect("service publication receipt")
            .version,
        published
    );

    FleetCoordinatorRegistryStore::import(durable.clone());
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::commit_compiled_initial_services_for_test(
            active_version.clone(),
            operation_id,
            plan_hash,
            configuration_digest,
            receipt_hashes,
            vec![service],
        )
        .expect("exact publication retry after restart"),
        published
    );
    let conflict =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::commit_compiled_initial_services_for_test(
            active_version,
            operation_id,
            plan_hash,
            configuration_digest,
            vec![[48; 32], [47; 32]],
            current.registry.services.clone(),
        )
        .expect_err("conflicting root receipt evidence must not replay");
    assert_eq!(
        conflict.public_error().map(|error| error.code),
        Some(ErrorCode::Conflict)
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable);

    let mut corrupted = durable;
    corrupted
        .current
        .as_mut()
        .expect("Coordinator state")
        .service_publication_receipt
        .as_mut()
        .expect("service receipt")
        .version
        .content_hash[0] ^= 1;
    FleetCoordinatorRegistryStore::import(corrupted);
    let invalid = crate::api::fleet_coordinator::FleetCoordinatorApi::registry()
        .expect_err("corrupted service publication history must fail closed");
    assert_eq!(invalid.code, ErrorCode::InvariantViolation);
}

#[test]
fn service_publication_history_precedes_later_unrelated_root_draining() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let (first, second, active_version) = activate_two_roots(principal(70));
    let service = FleetServiceBinding {
        service: "projects".parse().expect("service ID"),
        role: "project".parse().expect("role"),
        component_spec: "projects".parse().expect("Component Spec ID"),
        mode: FleetServiceMode::AuthorityReplica,
        placement: FleetServicePlacementPolicy {
            maximum_members_per_root: 1,
            minimum_distinct_roots: 1,
        },
        members: vec![service_member(
            FleetServiceMemberPurpose::Authority,
            71,
            first.fleet_subnet_root,
            72,
            "authority",
        )],
    };
    let published =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::commit_compiled_initial_services_for_test(
            active_version,
            [73; 32],
            [74; 32],
            ComponentDeploymentConfigurationDigest::from_bytes([75; 32]),
            vec![[76; 32], [77; 32]],
            vec![service.clone()],
        )
        .expect("publish service topology");
    let draining = FleetCoordinatorWorkflow::publish_root_draining(
        FleetSubnetRootDrainingPublicationRequest {
            expected_registry: published.clone(),
            root_draining: FleetSubnetRootDrainingResponse {
                operation_id: [78; 32],
                fleet_subnet_root: second.fleet_subnet_root,
                placement_subnet: second.placement_subnet,
                active_registry: published.clone(),
                component_topology_digest: second.component_topology_digest,
                active_release_set: second.active_release_set,
                next_allocation_sequence: 1,
                reserved_component_instances: 0,
                committed_component_instances: 0,
                managed_descendants: 0,
                known_created_component_canisters: 0,
                root_registry_encoded_bytes: 0,
                started_at_ns: 79,
            },
        },
    )
    .expect("drain root outside the published service");
    assert_eq!(draining.previous_version, published);

    let durable = FleetCoordinatorRegistryStore::export();
    FleetCoordinatorRegistryStore::import(durable);
    let registry = FleetCoordinatorWorkflow::registry().expect("reconstructed Registry history");
    assert_eq!(registry.services, vec![service]);
    assert_eq!(
        registry
            .fleet_subnet_roots
            .iter()
            .find(|root| root.fleet_subnet_root == second.fleet_subnet_root)
            .expect("unrelated root")
            .status,
        FleetSubnetRootStatus::Draining
    );
}

#[test]
fn ordinary_only_provisioning_records_publication_without_registry_mutation() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let (_, _, active_version) = activate_two_roots(principal(80));

    let observed =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::commit_compiled_initial_services_for_test(
            active_version.clone(),
            [81; 32],
            [82; 32],
            ComponentDeploymentConfigurationDigest::from_bytes([83; 32]),
            vec![[84; 32], [85; 32]],
            Vec::new(),
        )
        .expect("record service-free publication boundary");

    assert_eq!(observed, active_version);
    let current = FleetCoordinatorRegistryStore::export()
        .current
        .expect("Coordinator state");
    assert!(current.registry.services.is_empty());
    assert!(
        current
            .service_publication_receipt
            .expect("publication receipt")
            .services
            .is_empty()
    );
}

#[test]
fn complete_component_plan_is_durable_before_root_effects_and_replays_exactly() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let config = coordinator_config();
    let (_, _, _) = activate_two_roots(principal(90));
    let registry = FleetCoordinatorWorkflow::registry().expect("active Registry");
    let plan = fresh_component_plan(&config, &registry);
    let plan_hash =
        ComponentProvisioningPlanOps::hash(&config, &registry, &plan).expect("canonical plan hash");
    let request = FleetComponentProvisioningPrepareRequest {
        operation_id: [91; 32],
        plan,
    };

    assert_invalid_plan_identity_rejects_before_persistence(&config, &request);
    let prepared = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(&config, request.clone(), 92)
        .expect("persist complete plan");
    assert_prepared_plan_summary(&prepared, plan_hash);

    let durable = FleetCoordinatorRegistryStore::export();
    FleetCoordinatorRegistryStore::import(durable.clone());
    assert_prepared_plan_replays_exactly(&config, &request, plan_hash, &prepared);
    assert_conflicting_plan_authority_fails_closed(&config, request, plan_hash, &durable);
}

fn assert_invalid_plan_identity_rejects_before_persistence(
    config: &ConfigModel,
    request: &FleetComponentProvisioningPrepareRequest,
) {
    let mut zero_operation = request.clone();
    zero_operation.operation_id = [0; 32];
    let invalid = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(&config, zero_operation, 92)
        .expect_err("zero operation ID must reject before persistence");
    assert_eq!(
        invalid.public_error().map(|error| error.code),
        Some(ErrorCode::InvalidInput)
    );
    assert!(
        FleetCoordinatorRegistryStore::export()
            .current
            .expect("Coordinator state")
            .component_provisioning
            .is_none()
    );
}

fn assert_prepared_plan_summary(
    prepared: &FleetComponentProvisioningStatusResponse,
    plan_hash: [u8; 32],
) {
    assert_eq!(prepared.plan_hash, plan_hash);
    assert_eq!(prepared.phase, FleetComponentProvisioningPhase::Planned);
    assert_eq!(prepared.directory_confirmation_root_count, 2);
    assert_eq!(prepared.root_batch_count, 2);
    assert_eq!(prepared.group_placement_count, 2);
    assert_eq!(prepared.component_count, 2);
    assert_eq!(prepared.planned_at_ns, 92);
}

fn assert_prepared_plan_replays_exactly(
    config: &ConfigModel,
    request: &FleetComponentProvisioningPrepareRequest,
    plan_hash: [u8; 32],
    prepared: &FleetComponentProvisioningStatusResponse,
) {
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::component_provisioning_status_for_test(
            config,
            FleetComponentProvisioningStatusRequest {
                operation_id: request.operation_id,
                plan_hash,
            },
        )
        .expect("status after restart"),
        prepared.clone()
    );
    let wrong_status =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::component_provisioning_status_for_test(
            config,
            FleetComponentProvisioningStatusRequest {
                operation_id: request.operation_id,
                plan_hash: [93; 32],
            },
        )
        .expect_err("status cannot cross protected plan authority");
    assert_eq!(
        wrong_status.public_error().map(|error| error.code),
        Some(ErrorCode::Conflict)
    );
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            prepare_component_provisioning_for_test(config, request.clone(), 999)
            .expect("exact preparation retry"),
        prepared.clone(),
        "an exact retry preserves the original durable preparation time"
    );
}

fn assert_conflicting_plan_authority_fails_closed(
    config: &ConfigModel,
    request: FleetComponentProvisioningPrepareRequest,
    plan_hash: [u8; 32],
    durable: &FleetCoordinatorRegistryData,
) {
    let mut conflicting = request;
    conflicting.plan.directory_confirmation_roots.pop();
    let conflict = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(config, conflicting, 93)
        .expect_err("one operation cannot replace its complete plan");
    assert_eq!(
        conflict.public_error().map(|error| error.code),
        Some(ErrorCode::Conflict)
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable.clone());

    let drain =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::require_root_lifecycle_open_for_test(
            config,
        )
        .expect_err("a planned grouped Fleet fences root lifecycle");
    assert_eq!(
        drain.public_error().map(|error| error.code),
        Some(ErrorCode::Conflict)
    );

    let mut corrupted = durable.clone();
    corrupted
        .current
        .as_mut()
        .expect("Coordinator state")
        .component_provisioning
        .as_mut()
        .expect("provisioning record")
        .plan_hash[0] ^= 1;
    FleetCoordinatorRegistryStore::import(corrupted);
    let invalid =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::component_provisioning_status_for_test(
            config,
            FleetComponentProvisioningStatusRequest {
                operation_id: [91; 32],
                plan_hash,
            },
        )
        .expect_err("corrupt durable plan authority must fail closed");
    assert_eq!(invalid.class(), InternalErrorClass::Invariant);
    FleetCoordinatorRegistryStore::import(durable.clone());
}

fn activate_two_roots(
    coordinator: Principal,
) -> (
    FleetSubnetRootEntry,
    FleetSubnetRootEntry,
    FleetRegistryVersion,
) {
    let args = init_args(coordinator);
    let topology = args.component_topology.clone();
    FleetCoordinatorWorkflow::initialize(args, principal(60), true, coordinator)
        .expect("commit genesis");
    let first = joining_entry(&topology, 61, 62, 1);
    let second = joining_entry(&topology, 63, 64, 1);
    let first_join = FleetCoordinatorWorkflow::join_root(FleetSubnetRootJoinRequest {
        expected_registry: FleetCoordinatorWorkflow::version().expect("genesis version"),
        entry: first.clone(),
    })
    .expect("join first root");
    let second_join = FleetCoordinatorWorkflow::join_root(FleetSubnetRootJoinRequest {
        expected_registry: first_join.version,
        entry: second.clone(),
    })
    .expect("join second root");
    for root in [&first, &second] {
        FleetCoordinatorWorkflow::acknowledge_root_snapshot(
            root.fleet_subnet_root,
            canic_core::dto::fleet_registry::FleetSubnetRootSnapshotAcknowledgementRequest {
                version: second_join.version.clone(),
            },
        )
        .expect("acknowledge joining snapshot");
    }
    let active = FleetCoordinatorWorkflow::activate_registry(FleetRegistryActivationRequest {
        expected_registry: second_join.version,
    })
    .expect("activate roots");
    (first, second, active.version)
}

fn fresh_component_plan(
    config: &ConfigModel,
    registry: &FleetRegistry,
) -> FleetComponentProvisioningPlan {
    let component_topology = config
        .compile_component_topology()
        .expect("Component Topology");
    let deployment_topology = config
        .compile_component_group_deployment_topology()
        .expect("Component deployment topology");
    let deployment = deployment_topology
        .get(&"project_cells".parse().expect("deployment ID"))
        .expect("project cells deployment");
    let entries = deployment
        .members
        .iter()
        .map(|member| ComponentGroupPlanEntry {
            member_path: member.member_path.clone(),
            component_spec: member.component_spec.clone(),
            spec_hash: member.component_spec_hash,
            purpose: member.purpose.clone(),
            labels: member.labels.clone(),
            limits: member.limits.clone(),
        })
        .collect::<Vec<_>>();
    let batches = registry
        .fleet_subnet_roots
        .iter()
        .enumerate()
        .map(|(ordinal, root)| FleetSubnetRootProvisioningBatch {
            root: FleetSubnetRootBinding {
                authority: registry.authority.clone(),
                placement_subnet: root.placement_subnet,
                fleet_subnet_root: root.fleet_subnet_root,
                component_admissions: root.component_admissions.clone(),
                component_topology_digest: root.component_topology_digest,
                limits: root.limits.clone(),
            },
            active_release_set: root.active_release_set,
            placements: vec![ComponentGroupPlacementPlan {
                group_placement: ComponentGroupPlacementId {
                    deployment: deployment.deployment.clone(),
                    ordinal: u32::try_from(ordinal).expect("bounded placement ordinal"),
                },
                component_group: deployment.component_group.clone(),
                entries: entries.clone(),
            }],
        })
        .collect::<Vec<_>>();
    let mut directory_confirmation_roots = registry
        .fleet_subnet_roots
        .iter()
        .map(|root| root.fleet_subnet_root)
        .collect::<Vec<_>>();
    directory_confirmation_roots.sort_unstable();
    FleetComponentProvisioningPlan {
        fleet: registry.authority.binding.fleet.clone(),
        fleet_registry: FleetRegistryOps::version(
            &registry.authority,
            &component_topology,
            registry,
        )
        .expect("active Registry version"),
        configuration_digest: config
            .compile_component_deployment_configuration_digest()
            .expect("configuration digest"),
        operation: FleetComponentProvisioningOperation::FreshInstall,
        directory_confirmation_roots,
        batches,
    }
}

fn service_member(
    member_purpose: FleetServiceMemberPurpose,
    component_byte: u8,
    fleet_subnet_root: Principal,
    canister_byte: u8,
    deployment: &str,
) -> FleetServiceComponentBinding {
    FleetServiceComponentBinding {
        member_purpose,
        component: ComponentInstanceId::from_generated_bytes([component_byte; 32]),
        fleet_subnet_root,
        canister_id: principal(canister_byte),
        group_placement: ComponentGroupPlacementId {
            deployment: deployment.parse().expect("deployment ID"),
            ordinal: 0,
        },
        member_path: ComponentGroupMemberPath::try_from(vec![
            "project".parse().expect("member ID"),
        ])
        .expect("member path"),
    }
}

fn assert_snapshot_acknowledgements(
    registry: &FleetRegistry,
    first_entry: &FleetSubnetRootEntry,
    second_entry: &FleetSubnetRootEntry,
    version: &FleetRegistryVersion,
) -> FleetRegistryVersion {
    let snapshot = FleetCoordinatorWorkflow::snapshot_for_root(first_entry.fleet_subnet_root)
        .expect("registered root snapshot");
    assert_eq!(&snapshot.registry, registry);
    assert_eq!(&snapshot.version, version);
    let unauthorized_snapshot = FleetCoordinatorWorkflow::snapshot_for_root(principal(99))
        .expect_err("unregistered caller cannot fetch snapshot");
    assert_eq!(
        unauthorized_snapshot.public_error().map(|error| error.code),
        Some(ErrorCode::Forbidden)
    );

    let request = canic_core::dto::fleet_registry::FleetSubnetRootSnapshotAcknowledgementRequest {
        version: version.clone(),
    };
    let first_ack = FleetCoordinatorWorkflow::acknowledge_root_snapshot(
        first_entry.fleet_subnet_root,
        request.clone(),
    )
    .expect("first acknowledgement");
    assert_eq!(
        FleetCoordinatorWorkflow::acknowledge_root_snapshot(
            first_entry.fleet_subnet_root,
            request.clone(),
        )
        .expect("exact acknowledgement retry"),
        first_ack
    );
    let activation_request = FleetRegistryActivationRequest {
        expected_registry: version.clone(),
    };
    let incomplete = FleetCoordinatorWorkflow::activate_registry(activation_request.clone())
        .expect_err("activation requires every root acknowledgement");
    assert_eq!(
        incomplete.public_error().map(|error| error.code),
        Some(ErrorCode::Conflict)
    );
    FleetCoordinatorWorkflow::acknowledge_root_snapshot(second_entry.fleet_subnet_root, request)
        .expect("second acknowledgement");
    let acknowledgements =
        FleetCoordinatorWorkflow::root_snapshot_acknowledgements().expect("acknowledgements");
    assert_eq!(acknowledgements.len(), 2);
    assert!(acknowledgements.iter().all(|ack| &ack.version == version));

    let activated = FleetCoordinatorWorkflow::activate_registry(activation_request.clone())
        .expect("activate complete acknowledged Registry");
    assert_eq!(&activated.previous_version, version);
    assert_eq!(activated.version.revision, version.revision + 1);
    assert_eq!(
        FleetCoordinatorWorkflow::activate_registry(activation_request)
            .expect("exact activation retry"),
        activated
    );
    let registry = FleetCoordinatorWorkflow::registry().expect("active Registry");
    assert!(
        registry
            .fleet_subnet_roots
            .iter()
            .all(|entry| entry.status == FleetSubnetRootStatus::Active)
    );
    assert!(
        FleetCoordinatorWorkflow::root_snapshot_acknowledgements()
            .expect("cleared acknowledgements")
            .is_empty()
    );
    activated.version
}

fn assert_root_draining_publication(
    first_entry: &FleetSubnetRootEntry,
    second_entry: &FleetSubnetRootEntry,
    active_version: &FleetRegistryVersion,
) {
    let request = FleetSubnetRootDrainingPublicationRequest {
        expected_registry: active_version.clone(),
        root_draining: FleetSubnetRootDrainingResponse {
            operation_id: [21; 32],
            fleet_subnet_root: first_entry.fleet_subnet_root,
            placement_subnet: first_entry.placement_subnet,
            active_registry: active_version.clone(),
            component_topology_digest: first_entry.component_topology_digest,
            active_release_set: first_entry.active_release_set,
            next_allocation_sequence: 3,
            reserved_component_instances: 1,
            committed_component_instances: 1,
            managed_descendants: 2,
            known_created_component_canisters: 3,
            root_registry_encoded_bytes: 1_024,
            started_at_ns: 22,
        },
    };
    let before_invalid = FleetCoordinatorRegistryStore::export();
    let mut oversized = request.clone();
    oversized.root_draining.root_registry_encoded_bytes =
        first_entry.limits.maximum_registry_bytes + 1;
    let invalid = FleetCoordinatorWorkflow::publish_root_draining(oversized)
        .expect_err("reject root draining receipt outside protected limits");
    assert_eq!(
        invalid.public_error().map(|error| error.code),
        Some(ErrorCode::InvalidInput)
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), before_invalid);

    let published = FleetCoordinatorWorkflow::publish_root_draining(request.clone())
        .expect("publish root Draining");
    assert_eq!(&published.previous_version, active_version);
    assert_eq!(published.version.revision, active_version.revision + 1);

    let durable = FleetCoordinatorRegistryStore::export();
    FleetCoordinatorRegistryStore::import(durable);
    assert_eq!(
        FleetCoordinatorWorkflow::publish_root_draining(request.clone())
            .expect("exact publication retry after restart"),
        published
    );
    let registry = FleetCoordinatorWorkflow::registry().expect("Draining Registry");
    assert_eq!(
        registry
            .fleet_subnet_roots
            .iter()
            .find(|entry| entry.fleet_subnet_root == first_entry.fleet_subnet_root)
            .expect("first root")
            .status,
        FleetSubnetRootStatus::Draining
    );
    assert_eq!(
        registry
            .fleet_subnet_roots
            .iter()
            .find(|entry| entry.fleet_subnet_root == second_entry.fleet_subnet_root)
            .expect("second root")
            .status,
        FleetSubnetRootStatus::Active
    );

    let mut conflicting = request;
    conflicting.root_draining.operation_id[0] ^= 1;
    let conflict = FleetCoordinatorWorkflow::publish_root_draining(conflicting)
        .expect_err("one root cannot publish different draining authority");
    assert_eq!(
        conflict.public_error().map(|error| error.code),
        Some(ErrorCode::Conflict)
    );

    let valid = FleetCoordinatorRegistryStore::export();
    let mut corrupted = valid.clone();
    corrupted
        .current
        .as_mut()
        .expect("Coordinator state")
        .root_draining_publication_receipts[0]
        .response
        .version
        .content_hash[0] ^= 1;
    FleetCoordinatorRegistryStore::import(corrupted);
    let invalid = crate::api::fleet_coordinator::FleetCoordinatorApi::registry()
        .expect_err("reject corrupted root Draining publication receipt");
    assert_eq!(invalid.code, ErrorCode::InvariantViolation);
    FleetCoordinatorRegistryStore::import(valid);

    assert_root_removal_publication(first_entry, second_entry, &published);
}

fn assert_root_removal_publication(
    first_entry: &FleetSubnetRootEntry,
    second_entry: &FleetSubnetRootEntry,
    published: &canic_core::dto::fleet_registry::FleetSubnetRootDrainingPublicationResponse,
) {
    let removal_request = FleetSubnetRootRemovalPublicationRequest {
        expected_registry: published.version.clone(),
        final_inventory: FleetSubnetRootFinalInventoryResponse {
            operation_id: [21; 32],
            fleet_subnet_root: first_entry.fleet_subnet_root,
            placement_subnet: first_entry.placement_subnet,
            registry: published.version.clone(),
            component_topology_digest: first_entry.component_topology_digest,
            active_release_set: first_entry.active_release_set,
            next_allocation_sequence: 3,
            removed_component_instances: 2,
            terminal_component_history_hash: [23; 32],
            root_registry_encoded_bytes: 1_024,
            wasm_store: principal(24),
            wasm_store_catalog_hash: [25; 32],
            wasm_store_catalog_entries: 2,
            wasm_store_occupied_bytes: 2_048,
            wasm_store_template_count: 2,
            wasm_store_release_count: 2,
            wasm_store_gc_prepared_at_secs: 26,
            finalized_at_ns: 27,
            inventory_hash: [28; 32],
        },
    };
    let before_unauthorized = FleetCoordinatorRegistryStore::export();
    let unauthorized = FleetCoordinatorWorkflow::publish_root_removed(
        second_entry.fleet_subnet_root,
        removal_request.clone(),
    )
    .expect_err("only the exact draining root can publish its removal");
    assert_eq!(
        unauthorized.public_error().map(|error| error.code),
        Some(ErrorCode::Forbidden)
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), before_unauthorized);

    let removed = FleetCoordinatorWorkflow::publish_root_removed(
        first_entry.fleet_subnet_root,
        removal_request.clone(),
    )
    .expect("publish root Removed");
    assert_eq!(removed.previous_version, published.version);
    assert_eq!(removed.version.revision, published.version.revision + 1);
    let durable = FleetCoordinatorRegistryStore::export();
    FleetCoordinatorRegistryStore::import(durable);
    assert_eq!(
        FleetCoordinatorWorkflow::publish_root_removed(
            first_entry.fleet_subnet_root,
            removal_request,
        )
        .expect("exact removal retry after restart"),
        removed
    );
    let registry = FleetCoordinatorWorkflow::registry().expect("Removed Registry");
    assert_eq!(
        registry
            .fleet_subnet_roots
            .iter()
            .find(|entry| entry.fleet_subnet_root == first_entry.fleet_subnet_root)
            .expect("removed root")
            .status,
        FleetSubnetRootStatus::Removed
    );
    assert_eq!(
        registry
            .fleet_subnet_roots
            .iter()
            .find(|entry| entry.fleet_subnet_root == second_entry.fleet_subnet_root)
            .expect("surviving root")
            .status,
        FleetSubnetRootStatus::Active
    );
    let removed_snapshot =
        FleetCoordinatorWorkflow::snapshot_for_root(first_entry.fleet_subnet_root)
            .expect_err("Removed root cannot fetch a later Registry snapshot");
    assert_eq!(
        removed_snapshot.public_error().map(|error| error.code),
        Some(ErrorCode::Forbidden)
    );
    let surviving_snapshot =
        FleetCoordinatorWorkflow::snapshot_for_root(second_entry.fleet_subnet_root)
            .expect("surviving root can fetch Registry containing Removed peer");
    assert_eq!(surviving_snapshot.registry, registry);
    assert_eq!(surviving_snapshot.version, removed.version);
    assert_root_deletion_lifecycle(first_entry, &removed);
    assert_later_root_can_drain_after_removal(second_entry, &removed.version);
}

fn assert_root_deletion_lifecycle(
    root: &FleetSubnetRootEntry,
    removal: &canic_core::dto::fleet_registry::FleetSubnetRootRemovalPublicationResponse,
) {
    let coordinator = removal.version.authority.binding.coordinator;
    let operation_id = removal.final_inventory.operation_id;
    let retained_cycles_target = FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES + 1;
    let intent_request = FleetSubnetRootDeletionReadinessIntentRequest {
        operation_id,
        fleet_subnet_root: root.fleet_subnet_root,
        final_inventory_hash: removal.final_inventory.inventory_hash,
        store_deletion_hash: [41; 32],
        observed_cycles_before_reclamation: 500_000_000_000,
        retained_cycles_target,
        observed_reserved_cycles: 0,
        observed_idle_cycles_burned_per_day: 86_400,
        observed_freezing_threshold_seconds: 1,
        prepared_at_ns: 28,
    };
    let intent =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::prepare_root_deletion_readiness(
            root.fleet_subnet_root,
            coordinator,
            intent_request.clone(),
            29,
        )
        .expect("prepare root-deletion readiness intent");
    assert_ne!(intent.intent_hash, [0; 32]);
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::prepare_root_deletion_readiness(
            root.fleet_subnet_root,
            coordinator,
            intent_request,
            999,
        )
        .expect("exact readiness-intent retry"),
        intent
    );

    let readiness_request = FleetSubnetRootDeletionReadinessRequest {
        operation_id,
        fleet_subnet_root: root.fleet_subnet_root,
        expected_intent_hash: intent.intent_hash,
        observed_cycles_after_reclamation: 90_000_000_000,
        cycles_reclaimed_at_ns: 30,
    };
    let readiness =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::record_root_deletion_readiness(
            root.fleet_subnet_root,
            coordinator,
            readiness_request,
            31,
        )
        .expect("record root-deletion readiness");
    assert_ne!(readiness.readiness_hash, [0; 32]);

    assert_root_deletion_execution(root, coordinator, operation_id, readiness.readiness_hash);
}

fn assert_root_deletion_execution(
    root: &FleetSubnetRootEntry,
    coordinator: Principal,
    operation_id: [u8; 32],
    readiness_hash: [u8; 32],
) {
    let executor = principal(4);
    let execution_request = FleetSubnetRootDeletionExecutionRequest {
        operation_id,
        fleet_subnet_root: root.fleet_subnet_root,
        expected_readiness_hash: readiness_hash,
        observed_module_hash: [42; 32],
        observed_controllers: vec![executor],
        observed_cycles_after_reclamation: 90_000_000_000,
        observed_reserved_cycles: 0,
        observed_idle_cycles_burned_per_day: 86_400,
        observed_freezing_threshold_seconds: 1,
    };
    let execution =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::begin_root_deletion_execution(
            executor,
            coordinator,
            execution_request,
            32,
        )
        .expect("begin external root deletion");
    assert_ne!(execution.execution_hash, [0; 32]);
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::root_deletion_execution_status(
            FleetSubnetRootDeletionStatusRequest {
                operation_id,
                fleet_subnet_root: root.fleet_subnet_root,
            },
        )
        .expect("read root deletion execution"),
        execution
    );

    let completion_request = FleetSubnetRootDeletionCompletionRequest {
        operation_id,
        fleet_subnet_root: root.fleet_subnet_root,
        expected_execution_hash: execution.execution_hash,
        observed_absent_at_ns: 33,
    };
    let deletion = crate::ops::fleet_coordinator::FleetCoordinatorOps::complete_root_deletion(
        executor,
        coordinator,
        completion_request,
        34,
    )
    .expect("complete external root deletion");
    assert_ne!(deletion.deletion_hash, [0; 32]);
    let durable = FleetCoordinatorRegistryStore::export();
    FleetCoordinatorRegistryStore::import(durable);
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::complete_root_deletion(
            executor,
            coordinator,
            completion_request,
            999,
        )
        .expect("exact root deletion retry after restart"),
        deletion
    );
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::root_deletion_status(
            FleetSubnetRootDeletionStatusRequest {
                operation_id,
                fleet_subnet_root: root.fleet_subnet_root,
            },
        )
        .expect("durable root deletion status"),
        deletion
    );
}

fn assert_later_root_can_drain_after_removal(
    second_entry: &FleetSubnetRootEntry,
    removed_version: &FleetRegistryVersion,
) {
    let request = FleetSubnetRootDrainingPublicationRequest {
        expected_registry: removed_version.clone(),
        root_draining: FleetSubnetRootDrainingResponse {
            operation_id: [31; 32],
            fleet_subnet_root: second_entry.fleet_subnet_root,
            placement_subnet: second_entry.placement_subnet,
            active_registry: removed_version.clone(),
            component_topology_digest: second_entry.component_topology_digest,
            active_release_set: second_entry.active_release_set,
            next_allocation_sequence: 1,
            reserved_component_instances: 0,
            committed_component_instances: 0,
            managed_descendants: 0,
            known_created_component_canisters: 0,
            root_registry_encoded_bytes: 0,
            started_at_ns: 32,
        },
    };
    let published = FleetCoordinatorWorkflow::publish_root_draining(request)
        .expect("publish later root Draining after removal");
    assert_eq!(published.previous_version, removed_version.clone());
    let registry = FleetCoordinatorWorkflow::registry().expect("interleaved lifecycle Registry");
    assert_eq!(
        registry
            .fleet_subnet_roots
            .iter()
            .find(|entry| entry.fleet_subnet_root == second_entry.fleet_subnet_root)
            .expect("later draining root")
            .status,
        FleetSubnetRootStatus::Draining
    );
}

fn joining_entry(
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    subnet_byte: u8,
    root_byte: u8,
    maximum_root_instances: u32,
) -> FleetSubnetRootEntry {
    let spec = topology
        .component_specs
        .first()
        .expect("one Component Spec");
    let component_admissions = vec![ComponentSpecAdmission {
        component_spec: spec.component_spec.clone(),
        spec_hash: spec.spec_hash,
        maximum_root_instances,
    }];
    let component_topology_digest = topology
        .project_for_admissions(&component_admissions)
        .expect("root topology")
        .digest()
        .expect("root topology digest");
    FleetSubnetRootEntry {
        placement_subnet: SubnetId::from_principal(principal(subnet_byte)),
        fleet_subnet_root: principal(root_byte),
        component_admissions,
        component_topology_digest,
        active_release_set: FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [18; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([root_byte; 32]),
        },
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 3,
            maximum_registry_bytes: 2_097_152,
            maximum_wasm_store_bytes: 268_435_456,
            maximum_group_placements: 16,
            canister_pool: canic_core::ids::FleetSubnetCanisterPoolConfig {
                minimum_size: 1,
                maximum_size: 10,
                canister_cycles: Cycles::new(5_000_000_000_000),
            },
            cycles_funding: CyclesFundingBudget {
                window_secs: 3_600,
                maximum_cycles: Cycles::new(2_000_000_000_000),
            },
        },
        status: FleetSubnetRootStatus::Joining,
    }
}
