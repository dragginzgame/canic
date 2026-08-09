//! Module: workflow::fleet_coordinator::tests
//!
//! Responsibility: qualify protected genesis commitment and canonical Coordinator queries.
//! Does not own: PocketIC installation or host effect-journal coverage.

use super::*;
use crate::storage::stable::fleet_coordinator::{
    FleetComponentProvisioningStateRecord, FleetCoordinatorRegistryData,
    FleetCoordinatorRegistryStore,
};
use crate::view::fleet_coordinator::{
    FleetComponentDirectoryConfirmationDisposition,
    FleetComponentProvisioningRootAcceptanceDisposition,
    FleetComponentProvisioningRootProvisionCallView,
    FleetComponentProvisioningRootProvisionDisposition, FleetComponentRuntimeActivationDisposition,
};
use canic_core::{
    bootstrap::parse_config_model,
    cdk::types::Cycles,
    control_plane_support::{
        config::ConfigModel,
        error::InternalErrorClass,
        ops::{
            component_provisioning_plan::ComponentProvisioningPlanOps,
            component_provisioning_receipt::{
                RootComponentProvisioningAcceptanceReceiptAuthority,
                RootComponentProvisioningProvisionedReceiptAuthority,
                RootComponentProvisioningPublishedReceiptAuthority,
                RootComponentProvisioningReceiptOps,
                RootComponentProvisioningRuntimesActiveReceiptAuthority,
            },
            fleet_registry::FleetRegistryOps,
            fleet_service_binding::FleetServiceBindingOps,
        },
    },
    dto::{
        component_provisioning::{
            ComponentDirectoryPublicationEvidence, ComponentGroupDirectory,
            ComponentGroupDirectoryMember, ComponentGroupDirectoryProvenance,
            ComponentGroupDirectoryPublicationEvidence, ComponentGroupPlacementPlan,
            ComponentGroupPlanEntry, FleetComponentProvisioningAdvanceRequest,
            FleetComponentProvisioningOperation, FleetComponentProvisioningPhase,
            FleetComponentProvisioningPlan, FleetComponentProvisioningPrepareRequest,
            FleetComponentProvisioningStatusRequest, FleetComponentProvisioningStatusResponse,
            FleetSubnetRootProvisioningBatch, RootComponentActivationEvidence,
            RootComponentProvisioningAcceptanceRequest, RootComponentProvisioningPhase,
            RootComponentProvisioningResult, RootComponentProvisioningStatusResponse,
            RootComponentPublicationEvidence, RootProvisionedGroupMember,
            RootProvisionedGroupPlacement,
        },
        error::ErrorCode,
        fleet_registry::{
            FleetRegistryActivationRequest, FleetSubnetRootDeletionCompletionRequest,
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
        AppId, CanonicalNetworkId, ComponentBinding, ComponentGroupPlacementId,
        ComponentInstanceId, ComponentSpecAdmission, CyclesFundingBudget, FleetBinding,
        FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetRootBinding,
        FleetSubnetRootLimits, FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce,
        ReleaseSetDigest, SubnetId,
    },
};

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

#[test]
fn scale_out_current_root_progress_remains_canonical_through_registry_commitment() {
    let reserving = FleetComponentProvisioningRootProgress {
        fleet_subnet_root: principal(9),
        component_count: 3,
        reserved_component_count: 2,
        claimed_component_count: 0,
        installed_component_count: 0,
        registry_committed_component_count: 0,
    };
    let reserved = FleetComponentProvisioningRootProgress {
        reserved_component_count: 3,
        ..reserving
    };
    let claimed = FleetComponentProvisioningRootProgress {
        claimed_component_count: 1,
        ..reserved
    };
    let completely_claimed = FleetComponentProvisioningRootProgress {
        claimed_component_count: 3,
        ..reserved
    };
    let installing = FleetComponentProvisioningRootProgress {
        installed_component_count: 2,
        ..completely_claimed
    };
    let installed = FleetComponentProvisioningRootProgress {
        installed_component_count: 3,
        ..completely_claimed
    };
    let committing = FleetComponentProvisioningRootProgress {
        registry_committed_component_count: 2,
        ..installed
    };
    let committed = FleetComponentProvisioningRootProgress {
        registry_committed_component_count: 3,
        ..installed
    };
    for progress in [
        reserving,
        reserved,
        claimed,
        completely_claimed,
        installing,
        installed,
        committing,
        committed,
    ] {
        validate_scale_out_current_root_progress(Some(progress))
            .expect("canonical current-root progress");
    }

    let claim_without_identity = FleetComponentProvisioningRootProgress {
        claimed_component_count: 1,
        ..reserving
    };
    assert!(validate_scale_out_current_root_progress(Some(claim_without_identity)).is_err());
    let install_without_claim = FleetComponentProvisioningRootProgress {
        installed_component_count: 1,
        ..reserved
    };
    assert!(validate_scale_out_current_root_progress(Some(install_without_claim)).is_err());
    let commit_without_install = FleetComponentProvisioningRootProgress {
        registry_committed_component_count: 1,
        ..completely_claimed
    };
    assert!(validate_scale_out_current_root_progress(Some(commit_without_install)).is_err());
    assert!(validate_scale_out_current_root_progress(None).is_err());
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

const ORDINARY_COORDINATOR_CONFIG: &str = r#"
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

[component_group_deployments.project_cells]
component_group = "project_cell"
initial_placements = 2
maximum_placements = 2
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 2
"#;

const SCALE_OUT_COORDINATOR_CONFIG: &str = r#"
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
initial_placements = 1
maximum_placements = 2
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[services.fleet.targets.projects]
role = "project"
component_spec = "projects"
mode = "active_pool"
placement.maximum_members_per_root = 1
placement.minimum_distinct_roots = 2
"#;

const ORDINARY_SCALE_OUT_COORDINATOR_CONFIG: &str = r#"
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

[component_group_deployments.project_cells]
component_group = "project_cell"
initial_placements = 1
maximum_placements = 2
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#;

fn coordinator_config() -> ConfigModel {
    parse_config_model(COORDINATOR_CONFIG).expect("valid Coordinator config")
}

fn ordinary_coordinator_config() -> ConfigModel {
    parse_config_model(ORDINARY_COORDINATOR_CONFIG).expect("valid ordinary Coordinator config")
}

fn scale_out_coordinator_config() -> ConfigModel {
    parse_config_model(SCALE_OUT_COORDINATOR_CONFIG).expect("valid scale-out Coordinator config")
}

fn ordinary_scale_out_coordinator_config() -> ConfigModel {
    parse_config_model(ORDINARY_SCALE_OUT_COORDINATOR_CONFIG)
        .expect("valid ordinary scale-out Coordinator config")
}

fn init_args(coordinator: Principal) -> FleetCoordinatorInitArgs {
    init_args_with_config(coordinator, &coordinator_config())
}

fn init_args_with_config(coordinator: Principal, config: &ConfigModel) -> FleetCoordinatorInitArgs {
    let component_deployment_configuration = config
        .compile_component_deployment_configuration()
        .expect("Component deployment configuration");
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
        component_deployment_configuration,
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

    let durable = FleetCoordinatorRegistryStore::export();
    let mut corrupted = durable.clone();
    corrupted
        .current
        .as_mut()
        .expect("Coordinator state")
        .component_deployment_configuration
        .component_topology
        .component_specs[0]
        .spec_hash[0] ^= 1;
    FleetCoordinatorRegistryStore::import(corrupted);
    let invalid = FleetCoordinatorWorkflow::registry()
        .expect_err("corrupt durable compiled configuration must fail closed");
    assert_eq!(invalid.class(), InternalErrorClass::Invariant);
    FleetCoordinatorRegistryStore::import(durable);
}

#[test]
fn root_join_compare_and_commit_retains_exact_response_receipts() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let coordinator = principal(13);
    let args = init_args(coordinator);
    let topology = args
        .component_deployment_configuration
        .component_topology
        .clone();
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
fn initial_service_publication_commits_registry_receipt_and_phase_atomically() {
    let (config, plan_hash) = prepare_two_root_acceptance_plan();
    let provisioned = drive_components_provisioned(&config, plan_hash);
    let request = root_provision_advance_request(&provisioned);
    let before = FleetCoordinatorRegistryStore::export();
    let source_version = provisioned.fleet_registry.clone();

    assert_service_publication_cursor(&config, &provisioned, request, &before);
    assert_invalid_service_publication_time(&provisioned, &request, &before);
    let (published, durable) = commit_initial_service_publication(
        &request,
        source_version,
        provisioned.components_provisioned_at_ns,
    );
    assert_service_publication_replay_and_corruption(
        &config, plan_hash, &request, &published, durable,
    );
}

#[test]
fn directory_confirmation_intent_preserves_service_publication_receipts() {
    let (config, plan_hash) = prepare_two_root_acceptance_plan();
    let provisioned = drive_components_provisioned(&config, plan_hash);
    let service_request = root_provision_advance_request(&provisioned);
    let (published, _) = commit_initial_service_publication(
        &service_request,
        provisioned.fleet_registry,
        provisioned.components_provisioned_at_ns,
    );
    let request = root_provision_advance_request(&published);
    let disposition = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        advance_component_directory_confirmation(&request, 161)
        .expect("persist first Directory confirmation intent");
    assert!(matches!(
        disposition,
        FleetComponentDirectoryConfirmationDisposition::Invoke(_)
    ));
    let durable = FleetCoordinatorRegistryStore::export();
    let current = durable.current.expect("Coordinator state");
    assert_eq!(current.service_publication_receipts.len(), 1);
    assert!(matches!(
        current
            .component_provisioning
            .expect("provisioning state")
            .state,
        FleetComponentProvisioningStateRecord::ConfirmingDirectories { .. }
    ));
    let status =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::component_provisioning_status_for_test(
            &config,
            FleetComponentProvisioningStatusRequest {
                operation_id: published.operation_id,
                plan_hash,
            },
        )
        .expect("read Directory confirmation status");
    let request = root_provision_advance_request(&status);
    let before_replay = FleetCoordinatorRegistryStore::export();
    let replay = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        advance_component_provisioning_root_acceptance_for_test(&config, request, 162)
        .expect("completed root acceptance remains observational");
    assert!(matches!(
        replay,
        FleetComponentProvisioningRootAcceptanceDisposition::Current(_)
    ));
    assert_eq!(FleetCoordinatorRegistryStore::export(), before_replay);
}

fn assert_service_publication_cursor(
    config: &ConfigModel,
    provisioned: &FleetComponentProvisioningStatusResponse,
    request: FleetComponentProvisioningAdvanceRequest,
    before: &FleetCoordinatorRegistryData,
) {
    let mut completed_root_retry = request;
    completed_root_retry.expected_provisioned_root_count -= 1;
    let final_root = before
        .current
        .as_ref()
        .expect("Coordinator state")
        .component_provisioning
        .as_ref()
        .expect("provisioning record");
    let FleetComponentProvisioningStateRecord::ComponentsProvisioned { provisions, .. } =
        &final_root.state
    else {
        panic!("root completion must precede service publication")
    };
    completed_root_retry.expected_current_root = Some(root_progress_for_test(
        &provisions.last().expect("final root").response,
    ));
    let FleetComponentProvisioningRootProvisionDisposition::Current(replayed) =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_provisioning_root_for_test(
                config,
                &completed_root_retry,
                160,
            )
            .expect("completed root command replay")
    else {
        panic!("completed root command must not advance service publication")
    };
    assert_eq!(*replayed, provisioned.clone());
    assert_eq!(FleetCoordinatorRegistryStore::export(), before.clone());
    assert!(matches!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_provisioning_root_for_test(config, &request, 160)
            .expect("exact publication cursor"),
        FleetComponentProvisioningRootProvisionDisposition::Publish
    ));
    assert_eq!(FleetCoordinatorRegistryStore::export(), before.clone());
}

fn assert_invalid_service_publication_time(
    provisioned: &FleetComponentProvisioningStatusResponse,
    request: &FleetComponentProvisioningAdvanceRequest,
    before: &FleetCoordinatorRegistryData,
) {
    let invalid = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        publish_component_provisioning_services(
            request,
            provisioned.components_provisioned_at_ns.expect("completion time") - 1,
        )
        .expect_err("publication cannot predate complete provisioning");
    assert_eq!(
        invalid.public_error().map(|error| error.code),
        Some(ErrorCode::InvalidInput)
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), before.clone());
}

fn commit_initial_service_publication(
    request: &FleetComponentProvisioningAdvanceRequest,
    source_version: FleetRegistryVersion,
    components_provisioned_at_ns: Option<u64>,
) -> (
    FleetComponentProvisioningStatusResponse,
    FleetCoordinatorRegistryData,
) {
    let published = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        publish_component_provisioning_services(request, 160)
        .expect("atomically publish the complete initial service topology");
    assert_eq!(
        published.phase,
        FleetComponentProvisioningPhase::ServiceTopologyPublished
    );
    assert_eq!(published.service_topology_published_at_ns, Some(160));
    assert_eq!(
        published.components_provisioned_at_ns,
        components_provisioned_at_ns
    );
    let published_registry = published
        .published_fleet_registry
        .clone()
        .expect("published Fleet Registry");
    assert_eq!(published_registry.revision, source_version.revision + 1);

    let durable = FleetCoordinatorRegistryStore::export();
    let current = durable.current.as_ref().expect("Coordinator state");
    assert_eq!(current.registry.services.len(), 1);
    let receipt = current
        .service_publication_receipts
        .first()
        .expect("service publication receipt");
    assert_eq!(receipt.previous_version, source_version);
    assert_eq!(receipt.version, published_registry);
    assert_eq!(receipt.services, current.registry.services);
    assert_eq!(receipt.root_receipt_content_hashes.len(), 2);
    (published, durable)
}

fn assert_service_publication_replay_and_corruption(
    config: &ConfigModel,
    plan_hash: [u8; 32],
    request: &FleetComponentProvisioningAdvanceRequest,
    published: &FleetComponentProvisioningStatusResponse,
    durable: FleetCoordinatorRegistryData,
) {
    FleetCoordinatorRegistryStore::import(durable.clone());
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            publish_component_provisioning_services(request, 999)
            .expect("exact publication retry after restart"),
        published.clone()
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable);

    let mut incomplete = durable.clone();
    incomplete
        .current
        .as_mut()
        .expect("Coordinator state")
        .service_publication_receipts
        .clear();
    FleetCoordinatorRegistryStore::import(incomplete);
    let invalid =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::component_provisioning_status_for_test(
            config,
            FleetComponentProvisioningStatusRequest {
                operation_id: request.operation_id,
                plan_hash,
            },
        )
        .expect_err("publication phase without its atomic receipt must fail closed");
    assert_eq!(invalid.class(), InternalErrorClass::Invariant);

    let mut duplicated = durable.clone();
    let current = duplicated.current.as_mut().expect("Coordinator state");
    let receipt = current
        .service_publication_receipts
        .first()
        .expect("service receipt")
        .clone();
    current.service_publication_receipts.push(receipt);
    FleetCoordinatorRegistryStore::import(duplicated);
    let invalid =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::component_provisioning_status_for_test(
            config,
            FleetComponentProvisioningStatusRequest {
                operation_id: request.operation_id,
                plan_hash,
            },
        )
        .expect_err("one operation cannot retain duplicate publication receipts");
    assert_eq!(invalid.class(), InternalErrorClass::Invariant);

    let mut corrupted = durable;
    corrupted
        .current
        .as_mut()
        .expect("Coordinator state")
        .service_publication_receipts
        .first_mut()
        .expect("service receipt")
        .root_receipt_content_hashes[0][0] ^= 1;
    FleetCoordinatorRegistryStore::import(corrupted);
    let invalid = crate::api::fleet_coordinator::FleetCoordinatorApi::registry()
        .expect_err("corrupted terminal publication evidence must fail closed");
    assert_eq!(invalid.code, ErrorCode::InvariantViolation);
}

#[test]
fn ordinary_only_provisioning_records_publication_without_registry_mutation() {
    let config = ordinary_coordinator_config();
    let plan_hash = prepare_two_root_acceptance_plan_with(&config);
    let provisioned = drive_components_provisioned(&config, plan_hash);
    let source_version = provisioned.fleet_registry.clone();
    let published = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        publish_component_provisioning_services(
            &root_provision_advance_request(&provisioned),
            160,
        )
        .expect("record service-free publication boundary");

    assert_eq!(published.published_fleet_registry, Some(source_version));
    let current = FleetCoordinatorRegistryStore::export()
        .current
        .expect("Coordinator state");
    assert!(current.registry.services.is_empty());
    assert!(
        current
            .service_publication_receipts
            .first()
            .expect("publication receipt")
            .services
            .is_empty()
    );
}

#[test]
fn scale_out_publishes_all_new_pool_members_in_one_atomic_registry_append() {
    let config = scale_out_coordinator_config();
    let fresh = drive_terminal_fresh_install(&config);
    assert_eq!(
        fresh.phase,
        FleetComponentProvisioningPhase::RuntimesActivated
    );
    let source_registry = FleetCoordinatorWorkflow::registry().expect("published Registry");
    let source_version = FleetCoordinatorWorkflow::version().expect("published version");
    assert_eq!(source_registry.services[0].members.len(), 1);

    let plan = one_placement_scale_out_plan(&config, &source_registry);
    let mut status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(
            &config,
            FleetComponentProvisioningPrepareRequest {
                operation_id: [201; 32],
                plan,
            },
            1_000,
        )
        .expect("prepare exact scale-out plan");
    status = drive_root_acceptance(&config, status, 1_010);
    status = drive_root_provisioning(&config, status, 1_020);
    assert_eq!(
        status.phase,
        FleetComponentProvisioningPhase::ComponentsProvisioned
    );
    let prepublication = FleetCoordinatorRegistryStore::export();
    let current = prepublication.current.as_ref().expect("Coordinator state");
    let record = current
        .component_scale_out
        .as_ref()
        .expect("scale-out operation");
    let root_receipts = match &record.state {
        FleetComponentProvisioningStateRecord::ComponentsProvisioned { provisions, .. } => {
            provisions
                .iter()
                .map(|record| record.response.clone())
                .collect::<Vec<_>>()
        }
        _ => panic!("scale-out roots must be terminal"),
    };
    FleetServiceBindingOps::compile_scale_out_compiled(
        &current.component_deployment_configuration,
        &current.registry,
        &record.plan,
        record.operation_id,
        record.plan_hash,
        &root_receipts,
    )
    .expect("compile exact scale-out service additions");
    let request = root_provision_advance_request(&status);
    let published = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        publish_component_provisioning_services(&request, 1_100)
        .expect("publish one atomic scale-out service append");

    assert_eq!(
        published.phase,
        FleetComponentProvisioningPhase::ServiceTopologyPublished
    );
    let published_version = published
        .published_fleet_registry
        .clone()
        .expect("published Registry version");
    assert_eq!(published_version.revision, source_version.revision + 1);
    let durable = FleetCoordinatorRegistryStore::export();
    let current = durable.current.as_ref().expect("Coordinator state");
    assert_eq!(current.registry.services[0].members.len(), 2);
    assert_eq!(current.service_publication_receipts.len(), 2);
    let scale_out_receipt = &current.service_publication_receipts[1];
    assert_eq!(scale_out_receipt.previous_version, source_version);
    assert_eq!(scale_out_receipt.version, published_version);
    assert_eq!(scale_out_receipt.services, current.registry.services);
    assert_eq!(scale_out_receipt.root_receipt_content_hashes.len(), 1);
    assert!(published.current_publication.is_none());
    assert_eq!(published.directory_confirmed_root_count, 0);

    FleetCoordinatorRegistryStore::import(durable.clone());
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            publish_component_provisioning_services(&request, 9_999)
            .expect("replay exact scale-out publication after restart"),
        published
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable);

    let mut corrupted = durable;
    corrupted
        .current
        .as_mut()
        .expect("Coordinator state")
        .service_publication_receipts[1]
        .services[0]
        .members
        .pop();
    FleetCoordinatorRegistryStore::import(corrupted);
    let invalid = FleetCoordinatorWorkflow::registry()
        .expect_err("scale-out publication cannot remove its appended member");
    assert_eq!(invalid.class(), InternalErrorClass::Invariant);
}

#[test]
fn ordinary_scale_out_records_publication_without_registry_mutation() {
    let config = ordinary_scale_out_coordinator_config();
    drive_terminal_fresh_install(&config);
    let source_registry = FleetCoordinatorWorkflow::registry().expect("published Registry");
    let source_version = FleetCoordinatorWorkflow::version().expect("published version");
    assert!(source_registry.services.is_empty());

    let plan = one_placement_scale_out_plan(&config, &source_registry);
    let mut status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(
            &config,
            FleetComponentProvisioningPrepareRequest {
                operation_id: [151; 32],
                plan,
            },
            1_000,
        )
        .expect("prepare ordinary scale-out plan");
    status = drive_root_acceptance(&config, status, 1_010);
    status = drive_root_provisioning(&config, status, 1_020);
    let request = root_provision_advance_request(&status);
    let published = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        publish_component_provisioning_services(&request, 1_100)
        .expect("record ordinary scale-out publication boundary");

    assert_eq!(
        published.phase,
        FleetComponentProvisioningPhase::ServiceTopologyPublished
    );
    assert_eq!(
        published.published_fleet_registry,
        Some(source_version.clone())
    );
    assert_eq!(published.directory_confirmed_root_count, 0);
    let durable = FleetCoordinatorRegistryStore::export();
    let current = durable.current.as_ref().expect("Coordinator state");
    assert_eq!(current.registry, source_registry);
    assert_eq!(current.service_publication_receipts.len(), 2);
    let receipt = &current.service_publication_receipts[1];
    assert_eq!(receipt.previous_version, source_version);
    assert_eq!(receipt.version, receipt.previous_version);
    assert!(receipt.services.is_empty());

    FleetCoordinatorRegistryStore::import(durable.clone());
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            publish_component_provisioning_services(&request, 9_999)
            .expect("replay ordinary scale-out publication"),
        published
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable);
}

fn drive_terminal_fresh_install(config: &ConfigModel) -> FleetComponentProvisioningStatusResponse {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let (_, _, _) = activate_two_roots_with_config(principal(200), config);
    let registry = FleetCoordinatorWorkflow::registry().expect("active Registry");
    let plan = initial_scale_out_component_plan(config, &registry);
    let mut status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(
            config,
            FleetComponentProvisioningPrepareRequest {
                operation_id: [101; 32],
                plan,
            },
            100,
        )
        .expect("prepare initial plan");
    status = drive_root_acceptance(config, status, 110);
    status = drive_root_provisioning(config, status, 120);
    status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        publish_component_provisioning_services(&root_provision_advance_request(&status), 200)
        .expect("publish initial service");
    status = drive_directory_confirmation(status, 210);
    drive_runtime_activation(status, 300)
}

fn drive_root_acceptance(
    config: &ConfigModel,
    mut status: FleetComponentProvisioningStatusResponse,
    mut now: u64,
) -> FleetComponentProvisioningStatusResponse {
    while status.accepted_root_count < status.root_batch_count {
        let request = root_provision_advance_request(&status);
        let call = expect_root_acceptance_call(
            crate::ops::fleet_coordinator::FleetCoordinatorOps::
                advance_component_provisioning_root_acceptance_for_test(config, request, now)
                .expect("persist root acceptance intent"),
            false,
        );
        status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
            record_component_provisioning_root_acceptance_for_test(
                config,
                request,
                accepted_root_response(&call.request, now + 1),
                now + 2,
            )
            .expect("record root acceptance");
        now += 3;
    }
    status
}

fn drive_root_provisioning(
    config: &ConfigModel,
    mut status: FleetComponentProvisioningStatusResponse,
    mut now: u64,
) -> FleetComponentProvisioningStatusResponse {
    while status.phase != FleetComponentProvisioningPhase::ComponentsProvisioned {
        let request = root_provision_advance_request(&status);
        let call = expect_root_provision_call(
            crate::ops::fleet_coordinator::FleetCoordinatorOps::
                advance_component_provisioning_root_for_test(config, &request, now)
                .expect("persist root provisioning intent"),
            false,
        );
        let response = next_root_provision_response(config, &call, now + 1);
        status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
            record_component_provisioning_root_for_test(config, &request, response, now + 2)
            .expect("record root provisioning response");
        now += 3;
    }
    status
}

fn drive_directory_confirmation(
    mut status: FleetComponentProvisioningStatusResponse,
    mut now: u64,
) -> FleetComponentProvisioningStatusResponse {
    while status.phase != FleetComponentProvisioningPhase::DirectoriesConfirmed {
        let request = root_provision_advance_request(&status);
        let FleetComponentDirectoryConfirmationDisposition::Invoke(call) =
            crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_directory_confirmation(&request, now)
            .expect("persist Directory confirmation intent")
        else {
            panic!("Directory confirmation must invoke the next root");
        };
        let response = terminal_directory_response(call.fleet_subnet_root, now + 1);
        status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
            record_component_directory_confirmation(&request, response, now + 2)
            .expect("record terminal Directory response");
        now += 3;
    }
    status
}

fn drive_runtime_activation(
    mut status: FleetComponentProvisioningStatusResponse,
    mut now: u64,
) -> FleetComponentProvisioningStatusResponse {
    while status.phase != FleetComponentProvisioningPhase::RuntimesActivated {
        let request = root_provision_advance_request(&status);
        let FleetComponentRuntimeActivationDisposition::Invoke(call) =
            crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_runtime_activation(&request, now)
            .expect("persist runtime activation intent")
        else {
            panic!("runtime activation must invoke the next root");
        };
        let response = next_runtime_activation_response(call.fleet_subnet_root, now, now + 1);
        status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
            record_component_runtime_activation(&request, &response, now + 2)
            .expect("record runtime activation response");
        now += 3;
    }
    status
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
        prepare_component_provisioning_for_test(config, zero_operation, 92)
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
    assert_eq!(prepared.accepted_root_count, 0);
    assert_eq!(prepared.acceptance_in_flight_root, None);
    assert_eq!(prepared.provisioned_root_count, 0);
    assert_eq!(prepared.current_root, None);
    assert_eq!(prepared.provisioning_in_flight_root, None);
    assert_eq!(prepared.group_placement_count, 2);
    assert_eq!(prepared.component_count, 2);
    assert_eq!(prepared.planned_at_ns, 92);
    assert_eq!(prepared.roots_accepted_at_ns, None);
    assert_eq!(prepared.components_provisioned_at_ns, None);
    assert_eq!(prepared.published_fleet_registry, None);
    assert_eq!(prepared.service_topology_published_at_ns, None);
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

#[test]
fn coordinator_journals_each_root_acceptance_and_reconciles_lost_responses() {
    let (config, plan_hash) = prepare_two_root_acceptance_plan();

    let first_request = root_acceptance_advance_request(plan_hash, 0);
    let first_call = expect_root_acceptance_call(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_provisioning_root_acceptance_for_test(
                &config,
                first_request,
                103,
            )
            .expect("journal first root intent"),
        false,
    );
    assert_first_root_intent_is_durable(&config, plan_hash, first_call.fleet_subnet_root);
    let durable_intent = FleetCoordinatorRegistryStore::export();
    FleetCoordinatorRegistryStore::import(durable_intent.clone());
    let reconciled_call = expect_root_acceptance_call(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_provisioning_root_acceptance_for_test(
                &config,
                first_request,
                999,
            )
            .expect("reconcile first root intent"),
        true,
    );
    assert_eq!(reconciled_call.request, first_call.request);
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable_intent);

    let early_response = accepted_root_response(&first_call.request, 102);
    let invalid_time = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_provisioning_root_acceptance_for_test(
            &config,
            first_request,
            early_response,
            105,
        )
        .expect_err("root acceptance cannot predate its durable call intent");
    assert_eq!(
        invalid_time.public_error().map(|error| error.code),
        Some(ErrorCode::InvalidInput)
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable_intent);

    let first_response = accepted_root_response(&first_call.request, 104);
    let first_status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_provisioning_root_acceptance_for_test(
            &config,
            first_request,
            first_response.clone(),
            105,
        )
        .expect("record first root acceptance");
    assert_root_acceptance_status(
        &first_status,
        FleetComponentProvisioningPhase::AcceptingRoots,
        1,
    );
    assert_first_root_acceptance_replays(&config, first_request, first_response, &first_status);

    accept_second_root_and_reject_substitution(&config, plan_hash);
    assert_corrupt_root_acceptance_fails_closed(&config, plan_hash);
}

#[test]
fn coordinator_advances_each_accepted_root_and_freezes_terminal_receipts() {
    let (config, plan_hash) = prepare_two_root_acceptance_plan();
    accept_every_planned_root(&config, plan_hash);
    let mut status = component_provisioning_status(&config, plan_hash);
    assert_eq!(status.phase, FleetComponentProvisioningPhase::RootsAccepted);
    assert_eq!(status.provisioned_root_count, 0);

    let mut now = 120_u64;
    while status.phase != FleetComponentProvisioningPhase::ComponentsProvisioned {
        let request = root_provision_advance_request(&status);
        let call = expect_root_provision_call(
            crate::ops::fleet_coordinator::FleetCoordinatorOps::
                advance_component_provisioning_root_for_test(&config, &request, now)
                .expect("persist exact root provisioning intent"),
            false,
        );
        let durable_intent = FleetCoordinatorRegistryStore::export();
        FleetCoordinatorRegistryStore::import(durable_intent.clone());
        let reconciled = expect_root_provision_call(
            crate::ops::fleet_coordinator::FleetCoordinatorOps::
                advance_component_provisioning_root_for_test(&config, &request, now + 99)
                .expect("reconcile exact root provisioning intent"),
            true,
        );
        assert_eq!(reconciled.request, call.request);
        assert_eq!(FleetCoordinatorRegistryStore::export(), durable_intent);

        let response = next_root_provision_response(&config, &call, now + 1);
        if status.provisioned_root_count == 0
            && status
                .current_root
                .is_some_and(|progress| progress.reserved_component_count == 0)
        {
            assert_invalid_root_provision_responses(
                &config,
                &request,
                &response,
                now,
                &durable_intent,
            );
        }
        let next = crate::ops::fleet_coordinator::FleetCoordinatorOps::
            record_component_provisioning_root_for_test(
                &config,
                &request,
                response.clone(),
                now + 2,
            )
            .expect("record exact root provisioning response");
        assert_eq!(
            crate::ops::fleet_coordinator::FleetCoordinatorOps::
                record_component_provisioning_root_for_test(
                    &config,
                    &request,
                    response,
                    now + 99,
                )
                .expect("replay response after Coordinator commit"),
            next
        );
        let FleetComponentProvisioningRootProvisionDisposition::Current(replayed) =
            crate::ops::fleet_coordinator::FleetCoordinatorOps::
                advance_component_provisioning_root_for_test(&config, &request, now + 99)
                .expect("advance retry returns current durable status")
        else {
            panic!("committed root provisioning step must replay current status")
        };
        assert_eq!(*replayed, next);
        status = next;
        now += 3;
    }

    assert_terminal_root_provisioning_status(&status, now);
    let durable = FleetCoordinatorRegistryStore::export();
    FleetCoordinatorRegistryStore::import(durable.clone());
    assert_eq!(component_provisioning_status(&config, plan_hash), status);

    let mut corrupted = durable;
    let record = corrupted
        .current
        .as_mut()
        .expect("Coordinator state")
        .component_provisioning
        .as_mut()
        .expect("provisioning record");
    let FleetComponentProvisioningStateRecord::ComponentsProvisioned { provisions, .. } =
        &mut record.state
    else {
        panic!("all root receipts must be terminal")
    };
    provisions[0].response.receipt_content_hash[0] ^= 1;
    FleetCoordinatorRegistryStore::import(corrupted);
    let invalid =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::component_provisioning_status_for_test(
            &config,
            FleetComponentProvisioningStatusRequest {
                operation_id: [101; 32],
                plan_hash,
            },
        )
        .expect_err("corrupt terminal root receipt must fail closed");
    assert_eq!(invalid.class(), InternalErrorClass::Invariant);
}

fn assert_terminal_root_provisioning_status(
    status: &FleetComponentProvisioningStatusResponse,
    now: u64,
) {
    assert_eq!(status.provisioned_root_count, status.root_batch_count);
    assert_eq!(status.current_root, None);
    assert_eq!(status.provisioning_in_flight_root, None);
    assert_eq!(status.components_provisioned_at_ns, Some(now - 1));
    let mut terminal_scale_out = status.clone();
    terminal_scale_out.operation = FleetComponentProvisioningOperation::ScaleOut {
        deployment: "project_cells".parse().expect("deployment ID"),
        previous_placements: 1,
        requested_placements: 2,
    };
    assert!(
        !scale_out_service_publication_is_complete(&terminal_scale_out)
            .expect("terminal roots require one atomic service publication")
    );
    terminal_scale_out.phase = FleetComponentProvisioningPhase::ServiceTopologyPublished;
    terminal_scale_out.published_fleet_registry = Some(terminal_scale_out.fleet_registry.clone());
    terminal_scale_out.service_topology_published_at_ns = Some(now);
    assert!(
        scale_out_service_publication_is_complete(&terminal_scale_out)
            .expect("published scale-out remains fenced before Directories")
    );
}

fn accept_every_planned_root(config: &ConfigModel, plan_hash: [u8; 32]) {
    for index in 0..2_u32 {
        let request = root_acceptance_advance_request(plan_hash, index);
        let call = expect_root_acceptance_call(
            crate::ops::fleet_coordinator::FleetCoordinatorOps::
                advance_component_provisioning_root_acceptance_for_test(
                    config,
                    request,
                    110 + u64::from(index) * 3,
                )
                .expect("persist root acceptance intent"),
            false,
        );
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            record_component_provisioning_root_acceptance_for_test(
                config,
                request,
                accepted_root_response(&call.request, 111 + u64::from(index) * 3),
                112 + u64::from(index) * 3,
            )
            .expect("record root acceptance");
    }
}

fn component_provisioning_status(
    config: &ConfigModel,
    plan_hash: [u8; 32],
) -> FleetComponentProvisioningStatusResponse {
    crate::ops::fleet_coordinator::FleetCoordinatorOps::component_provisioning_status_for_test(
        config,
        FleetComponentProvisioningStatusRequest {
            operation_id: [101; 32],
            plan_hash,
        },
    )
    .expect("Component provisioning status")
}

fn assert_invalid_root_provision_responses(
    config: &ConfigModel,
    request: &FleetComponentProvisioningAdvanceRequest,
    response: &RootComponentProvisioningStatusResponse,
    started_at_ns: u64,
    durable_intent: &FleetCoordinatorRegistryData,
) {
    let mut skipped = response.clone();
    skipped.claimed_component_count = 1;
    let conflict = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_provisioning_root_for_test(
            config,
            request,
            skipped,
            started_at_ns + 2,
        )
        .expect_err("root response cannot skip a provisioning cursor");
    assert_eq!(
        conflict.public_error().map(|error| error.code),
        Some(ErrorCode::Conflict)
    );
    let early = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_provisioning_root_for_test(
            config,
            request,
            response.clone(),
            started_at_ns - 1,
        )
        .expect_err("root response cannot predate its durable call intent");
    assert_eq!(
        early.public_error().map(|error| error.code),
        Some(ErrorCode::InvalidInput)
    );
    let mut substituted = response.clone();
    substituted.fleet_subnet_root = principal(200);
    let conflict = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_provisioning_root_for_test(
            config,
            request,
            substituted,
            started_at_ns + 2,
        )
        .expect_err("root response cannot substitute its protected root");
    assert_eq!(
        conflict.public_error().map(|error| error.code),
        Some(ErrorCode::Conflict)
    );
    assert_eq!(
        FleetCoordinatorRegistryStore::export(),
        durable_intent.clone()
    );
}

fn root_provision_advance_request(
    status: &FleetComponentProvisioningStatusResponse,
) -> FleetComponentProvisioningAdvanceRequest {
    FleetComponentProvisioningAdvanceRequest {
        operation_id: status.operation_id,
        plan_hash: status.plan_hash,
        expected_phase: status.phase,
        expected_accepted_root_count: status.accepted_root_count,
        expected_provisioned_root_count: status.provisioned_root_count,
        expected_current_root: status.current_root,
        expected_directory_confirmed_root_count: status.directory_confirmed_root_count,
        expected_current_publication: status.current_publication,
        expected_runtime_activated_root_count: status.runtime_activated_root_count,
        expected_current_activation: status.current_activation,
    }
}

const fn root_progress_for_test(
    response: &RootComponentProvisioningStatusResponse,
) -> canic_core::dto::component_provisioning::FleetComponentProvisioningRootProgress {
    canic_core::dto::component_provisioning::FleetComponentProvisioningRootProgress {
        fleet_subnet_root: response.fleet_subnet_root,
        component_count: response.component_count,
        reserved_component_count: response.reserved_component_count,
        claimed_component_count: response.claimed_component_count,
        installed_component_count: response.installed_component_count,
        registry_committed_component_count: response.registry_committed_component_count,
    }
}

fn expect_root_provision_call(
    disposition: FleetComponentProvisioningRootProvisionDisposition,
    reconcile: bool,
) -> FleetComponentProvisioningRootProvisionCallView {
    match (disposition, reconcile) {
        (FleetComponentProvisioningRootProvisionDisposition::Invoke(call), false)
        | (FleetComponentProvisioningRootProvisionDisposition::Reconcile(call), true) => call,
        _ => panic!("root provisioning disposition differs from expected call boundary"),
    }
}

fn prepare_two_root_acceptance_plan() -> (ConfigModel, [u8; 32]) {
    let config = coordinator_config();
    let plan_hash = prepare_two_root_acceptance_plan_with(&config);
    (config, plan_hash)
}

fn prepare_two_root_acceptance_plan_with(config: &ConfigModel) -> [u8; 32] {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let (_, _, _) = activate_two_roots_with_config(principal(100), config);
    let registry = FleetCoordinatorWorkflow::registry().expect("active Registry");
    let plan = fresh_component_plan(config, &registry);
    let plan_hash =
        ComponentProvisioningPlanOps::hash(config, &registry, &plan).expect("canonical plan hash");
    crate::ops::fleet_coordinator::FleetCoordinatorOps::prepare_component_provisioning_for_test(
        config,
        FleetComponentProvisioningPrepareRequest {
            operation_id: [101; 32],
            plan,
        },
        102,
    )
    .expect("prepare complete plan");
    plan_hash
}

fn drive_components_provisioned(
    config: &ConfigModel,
    plan_hash: [u8; 32],
) -> FleetComponentProvisioningStatusResponse {
    accept_every_planned_root(config, plan_hash);
    let mut status = component_provisioning_status(config, plan_hash);
    let mut now = 120_u64;
    while status.phase != FleetComponentProvisioningPhase::ComponentsProvisioned {
        let request = root_provision_advance_request(&status);
        let call = expect_root_provision_call(
            crate::ops::fleet_coordinator::FleetCoordinatorOps::
                advance_component_provisioning_root_for_test(config, &request, now)
                .expect("persist root provisioning intent"),
            false,
        );
        let response = next_root_provision_response(config, &call, now + 1);
        status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
            record_component_provisioning_root_for_test(config, &request, response, now + 2)
            .expect("record root provisioning response");
        now += 3;
    }
    status
}

fn assert_first_root_intent_is_durable(
    config: &ConfigModel,
    plan_hash: [u8; 32],
    fleet_subnet_root: Principal,
) {
    let in_flight =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::component_provisioning_status_for_test(
            config,
            FleetComponentProvisioningStatusRequest {
                operation_id: [101; 32],
                plan_hash,
            },
        )
        .expect("durable first root intent status");
    assert_eq!(
        in_flight.phase,
        FleetComponentProvisioningPhase::AcceptingRoots
    );
    assert_eq!(in_flight.accepted_root_count, 0);
    assert_eq!(in_flight.acceptance_in_flight_root, Some(fleet_subnet_root));
}

fn assert_first_root_acceptance_replays(
    config: &ConfigModel,
    request: FleetComponentProvisioningAdvanceRequest,
    response: RootComponentProvisioningStatusResponse,
    expected_status: &FleetComponentProvisioningStatusResponse,
) {
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            record_component_provisioning_root_acceptance_for_test(
                config, request, response, 999,
            )
            .expect("replay after Coordinator response commit"),
        expected_status.clone()
    );

    let replay = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        advance_component_provisioning_root_acceptance_for_test(config, request, 999)
        .expect("first root exact retry");
    let FleetComponentProvisioningRootAcceptanceDisposition::Current(replayed) = replay else {
        panic!("committed root acceptance must replay current status")
    };
    assert_eq!(replayed, *expected_status);
}

fn root_acceptance_advance_request(
    plan_hash: [u8; 32],
    expected_accepted_root_count: u32,
) -> FleetComponentProvisioningAdvanceRequest {
    FleetComponentProvisioningAdvanceRequest {
        operation_id: [101; 32],
        plan_hash,
        expected_phase: FleetComponentProvisioningPhase::Planned,
        expected_accepted_root_count,
        expected_provisioned_root_count: 0,
        expected_current_root: None,
        expected_directory_confirmed_root_count: 0,
        expected_current_publication: None,
        expected_runtime_activated_root_count: 0,
        expected_current_activation: None,
    }
}

fn expect_root_acceptance_call(
    disposition: FleetComponentProvisioningRootAcceptanceDisposition,
    reconcile: bool,
) -> crate::view::fleet_coordinator::FleetComponentProvisioningRootAcceptanceCallView {
    match (disposition, reconcile) {
        (FleetComponentProvisioningRootAcceptanceDisposition::Invoke(call), false)
        | (FleetComponentProvisioningRootAcceptanceDisposition::Reconcile(call), true) => call,
        _ => panic!("root acceptance disposition differs from expected call boundary"),
    }
}

fn accepted_root_response(
    request: &RootComponentProvisioningAcceptanceRequest,
    accepted_at_ns: u64,
) -> RootComponentProvisioningStatusResponse {
    let placement_count = u32::try_from(request.batch.placements.len()).expect("placement count");
    let component_count = request
        .batch
        .placements
        .iter()
        .map(|placement| u32::try_from(placement.entries.len()).expect("member count"))
        .sum();
    let receipt_content_hash = RootComponentProvisioningReceiptOps::acceptance_content_hash(
        RootComponentProvisioningAcceptanceReceiptAuthority {
            operation_id: request.operation_id,
            plan_hash: request.plan_hash,
            fleet_registry: &request.fleet_registry,
            configuration_digest: request.configuration_digest,
            batch: &request.batch,
            placement_count,
            component_count,
            accepted_at_ns,
        },
    )
    .expect("acceptance receipt hash");
    RootComponentProvisioningStatusResponse {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
        fleet_registry: request.fleet_registry.clone(),
        configuration_digest: request.configuration_digest,
        fleet_subnet_root: request.batch.root.fleet_subnet_root,
        phase: RootComponentProvisioningPhase::Accepted,
        placement_count,
        component_count,
        reserved_component_count: 0,
        claimed_component_count: 0,
        installed_component_count: 0,
        registry_committed_component_count: 0,
        published_component_count: 0,
        activated_component_count: 0,
        root_runtime_active: false,
        result: None,
        publication: None,
        activation: None,
        accepted_at_ns,
        provisioned_at_ns: None,
        published_at_ns: None,
        activation_started_at_ns: None,
        runtimes_activated_at_ns: None,
        receipt_content_hash,
    }
}

fn next_root_provision_response(
    config: &ConfigModel,
    call: &FleetComponentProvisioningRootProvisionCallView,
    observed_at_ns: u64,
) -> RootComponentProvisioningStatusResponse {
    let durable = FleetCoordinatorRegistryStore::export();
    let current = durable.current.as_ref().expect("Coordinator state");
    let record = current
        .component_provisioning
        .as_ref()
        .filter(|record| record.operation_id == call.request.operation_id)
        .or_else(|| {
            current
                .component_scale_out
                .as_ref()
                .filter(|record| record.operation_id == call.request.operation_id)
        })
        .expect("provisioning record");
    let root_index = record
        .plan
        .batches
        .iter()
        .position(|batch| batch.root.fleet_subnet_root == call.fleet_subnet_root)
        .expect("planned root index");
    let batch = &record.plan.batches[root_index];
    let acceptance = provisioning_acceptances(&record.state)[root_index]
        .response
        .clone();
    let component_count = acceptance.component_count;
    let request = call.request;
    if request.expected_reserved_component_count < component_count {
        let mut response = acceptance;
        response.reserved_component_count = request.expected_reserved_component_count + 1;
        return response;
    }
    if request.expected_claimed_component_count < component_count {
        let mut response = acceptance;
        response.reserved_component_count = component_count;
        response.claimed_component_count = request.expected_claimed_component_count + 1;
        return response;
    }
    if request.expected_installed_component_count < component_count {
        let mut response = acceptance;
        response.reserved_component_count = component_count;
        response.claimed_component_count = component_count;
        response.installed_component_count = request.expected_installed_component_count + 1;
        return response;
    }
    if request.expected_registry_committed_component_count < component_count {
        let mut response = acceptance;
        response.reserved_component_count = component_count;
        response.claimed_component_count = component_count;
        response.installed_component_count = component_count;
        response.registry_committed_component_count =
            request.expected_registry_committed_component_count + 1;
        return response;
    }
    provisioned_root_response(
        config,
        record,
        batch,
        acceptance.accepted_at_ns,
        observed_at_ns,
    )
}

fn provisioning_acceptances(
    state: &FleetComponentProvisioningStateRecord,
) -> &[crate::storage::stable::fleet_coordinator::FleetComponentProvisioningRootAcceptanceRecord] {
    match state {
        FleetComponentProvisioningStateRecord::RootsAccepted { acceptances, .. }
        | FleetComponentProvisioningStateRecord::ProvisioningRoots { acceptances, .. }
        | FleetComponentProvisioningStateRecord::ComponentsProvisioned { acceptances, .. }
        | FleetComponentProvisioningStateRecord::ServiceTopologyPublished { acceptances, .. }
        | FleetComponentProvisioningStateRecord::ConfirmingDirectories { acceptances, .. }
        | FleetComponentProvisioningStateRecord::DirectoriesConfirmed { acceptances, .. }
        | FleetComponentProvisioningStateRecord::ActivatingRuntimes { acceptances, .. }
        | FleetComponentProvisioningStateRecord::RuntimesActivated { acceptances, .. } => {
            acceptances
        }
        FleetComponentProvisioningStateRecord::Planned { .. }
        | FleetComponentProvisioningStateRecord::AcceptingRoots { .. } => {
            panic!("root provisioning requires complete acceptances")
        }
    }
}

fn provisioned_root_response(
    config: &ConfigModel,
    record: &crate::storage::stable::fleet_coordinator::FleetComponentProvisioningRecord,
    batch: &FleetSubnetRootProvisioningBatch,
    accepted_at_ns: u64,
    provisioned_at_ns: u64,
) -> RootComponentProvisioningStatusResponse {
    let topology = config
        .compile_component_topology()
        .expect("Component Topology");
    let root_index = record
        .plan
        .batches
        .iter()
        .position(|planned| planned.root.fleet_subnet_root == batch.root.fleet_subnet_root)
        .expect("root index");
    let identity_byte = record.operation_id[0]
        .checked_add(49)
        .and_then(|base| base.checked_add(u8::try_from(root_index).expect("root index byte")))
        .expect("test identity byte");
    let result = RootComponentProvisioningResult {
        placements: batch
            .placements
            .iter()
            .map(|placement| RootProvisionedGroupPlacement {
                group_placement: placement.group_placement.clone(),
                component_group: placement.component_group.clone(),
                members: placement
                    .entries
                    .iter()
                    .map(|entry| {
                        let spec = topology.get(&entry.component_spec).expect("known Spec");
                        RootProvisionedGroupMember {
                            member_path: entry.member_path.clone(),
                            component_spec: entry.component_spec.clone(),
                            purpose: entry.purpose.clone(),
                            limits: entry.limits.clone(),
                            binding: ComponentBinding {
                                authority: batch.root.authority.clone(),
                                component: ComponentInstanceId::from_generated_bytes(
                                    [identity_byte; 32],
                                ),
                                component_spec: entry.component_spec.clone(),
                                spec_hash: entry.spec_hash,
                                role: spec.component_role.clone(),
                                placement_subnet: batch.root.placement_subnet,
                                fleet_subnet_root: batch.root.fleet_subnet_root,
                                canister_id: principal(identity_byte),
                            },
                            component_registry_revision: u64::from(identity_byte),
                            component_registry_content_hash: [identity_byte; 32],
                        }
                    })
                    .collect(),
            })
            .collect(),
    };
    let (placement_count, component_count) =
        batch
            .placements
            .iter()
            .fold((0_u32, 0_u32), |(placements, components), placement| {
                (
                    placements + 1,
                    components + u32::try_from(placement.entries.len()).expect("member count"),
                )
            });
    let receipt_content_hash = RootComponentProvisioningReceiptOps::provisioned_content_hash(
        RootComponentProvisioningProvisionedReceiptAuthority {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            fleet_registry: &record.plan.fleet_registry,
            configuration_digest: record.plan.configuration_digest,
            root: &batch.root,
            result: &result,
            accepted_at_ns,
            provisioned_at_ns,
        },
    )
    .expect("terminal receipt hash");
    RootComponentProvisioningStatusResponse {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        fleet_registry: record.plan.fleet_registry.clone(),
        configuration_digest: record.plan.configuration_digest,
        fleet_subnet_root: batch.root.fleet_subnet_root,
        phase: RootComponentProvisioningPhase::Provisioned,
        placement_count,
        component_count,
        reserved_component_count: component_count,
        claimed_component_count: component_count,
        installed_component_count: component_count,
        registry_committed_component_count: component_count,
        published_component_count: 0,
        activated_component_count: 0,
        root_runtime_active: false,
        result: Some(result),
        publication: None,
        activation: None,
        accepted_at_ns,
        provisioned_at_ns: Some(provisioned_at_ns),
        published_at_ns: None,
        activation_started_at_ns: None,
        runtimes_activated_at_ns: None,
        receipt_content_hash,
    }
}

fn terminal_directory_response(
    fleet_subnet_root: Principal,
    published_at_ns: u64,
) -> RootComponentProvisioningStatusResponse {
    let durable = FleetCoordinatorRegistryStore::export();
    let current = durable.current.as_ref().expect("Coordinator state");
    let record = current
        .component_provisioning
        .as_ref()
        .expect("fresh provisioning record");
    let (root_index, previous, published_registry) = match &record.state {
        FleetComponentProvisioningStateRecord::ConfirmingDirectories {
            provisions,
            published_fleet_registry,
            confirmations,
            current,
            in_flight: Some(intent),
            ..
        } => {
            let root_index = usize::try_from(intent.root_index).expect("root index");
            assert_eq!(confirmations.len(), root_index);
            let previous = current.as_ref().map_or_else(
                || provisions[root_index].response.clone(),
                |record| record.response.clone(),
            );
            (root_index, previous, published_fleet_registry)
        }
        _ => panic!("Directory response requires an in-flight confirmation"),
    };
    let batch = &record.plan.batches[root_index];
    assert_eq!(batch.root.fleet_subnet_root, fleet_subnet_root);
    let result = previous.result.clone().expect("provisioned result");
    let component_directories = result
        .placements
        .iter()
        .flat_map(|placement| &placement.members)
        .map(|member| ComponentDirectoryPublicationEvidence {
            component: member.binding.component,
            content_hash: member.component_registry_content_hash,
        })
        .collect();
    let component_group_directories = batch
        .placements
        .iter()
        .zip(&result.placements)
        .map(|(planned, provisioned)| {
            let directory = component_group_directory(record, batch, planned, provisioned);
            ComponentGroupDirectoryPublicationEvidence {
                group_placement: provisioned.group_placement.clone(),
                content_hash:
                    RootComponentProvisioningReceiptOps::component_group_directory_content_hash(
                        &directory,
                    )
                    .expect("Component Group Directory hash"),
            }
        })
        .collect();
    let fleet_directory = FleetRegistryOps::directory_for_root(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &current.registry,
        fleet_subnet_root,
    )
    .expect("Fleet Directory");
    assert_eq!(fleet_directory.provenance.registry, *published_registry);
    let publication = RootComponentPublicationEvidence {
        fleet_registry: published_registry.clone(),
        fleet_directory_content_hash:
            RootComponentProvisioningReceiptOps::fleet_directory_content_hash(&fleet_directory)
                .expect("Fleet Directory hash"),
        component_directories,
        component_group_directories,
    };
    let receipt_content_hash = RootComponentProvisioningReceiptOps::published_content_hash(
        RootComponentProvisioningPublishedReceiptAuthority {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            configuration_digest: record.plan.configuration_digest,
            root: &batch.root,
            result: &result,
            publication: &publication,
            accepted_at_ns: previous.accepted_at_ns,
            provisioned_at_ns: previous.provisioned_at_ns.expect("provisioned time"),
            published_at_ns,
        },
    )
    .expect("published receipt hash");
    let mut response = previous;
    response.phase = RootComponentProvisioningPhase::Published;
    response.published_component_count = response.component_count;
    response.publication = Some(publication);
    response.published_at_ns = Some(published_at_ns);
    response.receipt_content_hash = receipt_content_hash;
    response
}

fn component_group_directory(
    record: &crate::storage::stable::fleet_coordinator::FleetComponentProvisioningRecord,
    batch: &FleetSubnetRootProvisioningBatch,
    planned: &ComponentGroupPlacementPlan,
    provisioned: &RootProvisionedGroupPlacement,
) -> ComponentGroupDirectory {
    assert_eq!(planned.group_placement, provisioned.group_placement);
    assert_eq!(planned.component_group, provisioned.component_group);
    assert_eq!(planned.entries.len(), provisioned.members.len());
    let members = planned
        .entries
        .iter()
        .zip(&provisioned.members)
        .map(|(entry, member)| {
            assert_eq!(entry.member_path, member.member_path);
            assert_eq!(entry.component_spec, member.component_spec);
            assert_eq!(entry.purpose, member.purpose);
            ComponentGroupDirectoryMember {
                member_path: member.member_path.clone(),
                component_spec: member.component_spec.clone(),
                purpose: member.purpose.clone(),
                labels: entry.labels.clone(),
                binding: member.binding.clone(),
            }
        })
        .collect();
    ComponentGroupDirectory {
        provenance: ComponentGroupDirectoryProvenance {
            authority: batch.root.authority.clone(),
            fleet_subnet_root: batch.root.fleet_subnet_root,
            group_placement: provisioned.group_placement.clone(),
            component_group: provisioned.component_group.clone(),
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            placement_receipt_content_hash:
                RootComponentProvisioningReceiptOps::group_placement_content_hash(
                    record.operation_id,
                    record.plan_hash,
                    &batch.root,
                    provisioned,
                )
                .expect("placement receipt hash"),
        },
        members,
    }
}

fn next_runtime_activation_response(
    fleet_subnet_root: Principal,
    activation_started_at_ns: u64,
    observed_at_ns: u64,
) -> RootComponentProvisioningStatusResponse {
    let durable = FleetCoordinatorRegistryStore::export();
    let current = durable.current.as_ref().expect("Coordinator state");
    let record = current
        .component_provisioning
        .as_ref()
        .expect("fresh provisioning record");
    let (
        root_index,
        publication,
        activated_component_count,
        component_count,
        durable_started_at_ns,
    ) = match &record.state {
        FleetComponentProvisioningStateRecord::ActivatingRuntimes {
            confirmations,
            activations,
            current,
            in_flight: Some(intent),
            ..
        } => {
            let root_index = activations.len();
            assert_eq!(
                usize::try_from(intent.root_index).expect("root index"),
                root_index
            );
            let publication = confirmations[root_index].response.clone();
            let activated_component_count = current
                .as_ref()
                .map_or(publication.activated_component_count, |record| {
                    record.progress.activated_component_count
                });
            let component_count = current
                .as_ref()
                .map_or(publication.component_count, |record| {
                    record.progress.component_count
                });
            let durable_started_at_ns = current
                .as_ref()
                .and_then(|record| record.activation_started_at_ns)
                .unwrap_or(activation_started_at_ns);
            (
                root_index,
                publication,
                activated_component_count,
                component_count,
                durable_started_at_ns,
            )
        }
        _ => panic!("runtime response requires an in-flight activation"),
    };
    assert_eq!(publication.fleet_subnet_root, fleet_subnet_root);
    if activated_component_count < component_count {
        let mut response = publication;
        response.activated_component_count = activated_component_count + 1;
        response.activation_started_at_ns = Some(durable_started_at_ns);
        return response;
    }

    let batch = &record.plan.batches[root_index];
    let activation = RootComponentActivationEvidence {
        fleet_activation_operation_id: [fleet_subnet_root.as_slice()[0]; 32],
        initial_inventory_hash: [fleet_subnet_root.as_slice()[1]; 32],
        component_count,
        root_activated_at_ns: observed_at_ns,
    };
    let receipt_content_hash = RootComponentProvisioningReceiptOps::runtimes_active_content_hash(
        RootComponentProvisioningRuntimesActiveReceiptAuthority {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            configuration_digest: record.plan.configuration_digest,
            root: &batch.root,
            published_receipt_content_hash: publication.receipt_content_hash,
            activation,
            activation_started_at_ns: durable_started_at_ns,
            runtimes_activated_at_ns: observed_at_ns,
        },
    )
    .expect("runtime-active receipt hash");
    let mut response = publication;
    response.phase = RootComponentProvisioningPhase::RuntimesActive;
    response.activated_component_count = response.component_count;
    response.root_runtime_active = true;
    response.activation = Some(activation);
    response.activation_started_at_ns = Some(durable_started_at_ns);
    response.runtimes_activated_at_ns = Some(observed_at_ns);
    response.receipt_content_hash = receipt_content_hash;
    response
}

fn assert_root_acceptance_status(
    status: &FleetComponentProvisioningStatusResponse,
    phase: FleetComponentProvisioningPhase,
    accepted_root_count: u32,
) {
    assert_eq!(status.phase, phase);
    assert_eq!(status.accepted_root_count, accepted_root_count);
    assert_eq!(status.acceptance_in_flight_root, None);
}

fn accept_second_root_and_reject_substitution(config: &ConfigModel, plan_hash: [u8; 32]) {
    let request = root_acceptance_advance_request(plan_hash, 1);
    let call = expect_root_acceptance_call(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_provisioning_root_acceptance_for_test(config, request, 106)
            .expect("journal second root intent"),
        false,
    );
    let durable_intent = FleetCoordinatorRegistryStore::export();
    let mut substituted = accepted_root_response(&call.request, 107);
    substituted.fleet_subnet_root = principal(108);
    let conflict = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_provisioning_root_acceptance_for_test(
            config,
            request,
            substituted,
            108,
        )
        .expect_err("substituted root response must reject");
    assert_eq!(
        conflict.public_error().map(|error| error.code),
        Some(ErrorCode::Conflict)
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable_intent);

    let complete = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_provisioning_root_acceptance_for_test(
            config,
            request,
            accepted_root_response(&call.request, 107),
            109,
        )
        .expect("record second root acceptance");
    assert_root_acceptance_status(&complete, FleetComponentProvisioningPhase::RootsAccepted, 2);
    assert_eq!(complete.roots_accepted_at_ns, Some(109));
}

fn assert_corrupt_root_acceptance_fails_closed(config: &ConfigModel, plan_hash: [u8; 32]) {
    let exact = FleetCoordinatorRegistryStore::export();
    let mut corrupted = exact.clone();
    let record = corrupted
        .current
        .as_mut()
        .expect("Coordinator state")
        .component_provisioning
        .as_mut()
        .expect("provisioning record");
    let FleetComponentProvisioningStateRecord::RootsAccepted { acceptances, .. } =
        &mut record.state
    else {
        panic!("two accepted roots must be terminal")
    };
    acceptances[0].response.receipt_content_hash[0] ^= 1;
    FleetCoordinatorRegistryStore::import(corrupted);
    let invalid =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::component_provisioning_status_for_test(
            config,
            FleetComponentProvisioningStatusRequest {
                operation_id: [101; 32],
                plan_hash,
            },
        )
        .expect_err("corrupt accepted root evidence must fail closed");
    assert_eq!(invalid.class(), InternalErrorClass::Invariant);
    FleetCoordinatorRegistryStore::import(exact);
}

fn activate_two_roots(
    coordinator: Principal,
) -> (
    FleetSubnetRootEntry,
    FleetSubnetRootEntry,
    FleetRegistryVersion,
) {
    activate_two_roots_with_config(coordinator, &coordinator_config())
}

fn activate_two_roots_with_config(
    coordinator: Principal,
    config: &ConfigModel,
) -> (
    FleetSubnetRootEntry,
    FleetSubnetRootEntry,
    FleetRegistryVersion,
) {
    let args = init_args_with_config(coordinator, config);
    let topology = args
        .component_deployment_configuration
        .component_topology
        .clone();
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

fn initial_scale_out_component_plan(
    config: &ConfigModel,
    registry: &FleetRegistry,
) -> FleetComponentProvisioningPlan {
    let (deployment, component_group, entries) = project_cell_plan_entries(config);
    let batches = registry
        .fleet_subnet_roots
        .iter()
        .enumerate()
        .map(|(index, root)| FleetSubnetRootProvisioningBatch {
            root: fleet_subnet_root_binding(registry, root),
            active_release_set: root.active_release_set,
            placements: (index == 0)
                .then(|| ComponentGroupPlacementPlan {
                    group_placement: ComponentGroupPlacementId {
                        deployment: deployment.clone(),
                        ordinal: 0,
                    },
                    component_group: component_group.clone(),
                    entries: entries.clone(),
                })
                .into_iter()
                .collect(),
        })
        .collect();
    component_plan(
        config,
        registry,
        FleetComponentProvisioningOperation::FreshInstall,
        batches,
    )
}

fn one_placement_scale_out_plan(
    config: &ConfigModel,
    registry: &FleetRegistry,
) -> FleetComponentProvisioningPlan {
    let (deployment, component_group, entries) = project_cell_plan_entries(config);
    let root = &registry.fleet_subnet_roots[1];
    let mut plan = component_plan(
        config,
        registry,
        FleetComponentProvisioningOperation::ScaleOut {
            deployment: deployment.clone(),
            previous_placements: 1,
            requested_placements: 2,
        },
        vec![FleetSubnetRootProvisioningBatch {
            root: fleet_subnet_root_binding(registry, root),
            active_release_set: root.active_release_set,
            placements: vec![ComponentGroupPlacementPlan {
                group_placement: ComponentGroupPlacementId {
                    deployment,
                    ordinal: 1,
                },
                component_group,
                entries,
            }],
        }],
    );
    if registry.services.is_empty() {
        plan.directory_confirmation_roots = vec![root.fleet_subnet_root];
    }
    plan
}

fn project_cell_plan_entries(
    config: &ConfigModel,
) -> (
    canic_core::ids::ComponentGroupDeploymentId,
    canic_core::ids::ComponentGroupSpecId,
    Vec<ComponentGroupPlanEntry>,
) {
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
        .collect();
    (
        deployment.deployment.clone(),
        deployment.component_group.clone(),
        entries,
    )
}

fn component_plan(
    config: &ConfigModel,
    registry: &FleetRegistry,
    operation: FleetComponentProvisioningOperation,
    batches: Vec<FleetSubnetRootProvisioningBatch>,
) -> FleetComponentProvisioningPlan {
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
            &config
                .compile_component_topology()
                .expect("Component Topology"),
            registry,
        )
        .expect("Registry version"),
        configuration_digest: config
            .compile_component_deployment_configuration_digest()
            .expect("configuration digest"),
        operation,
        directory_confirmation_roots,
        batches,
    }
}

fn fleet_subnet_root_binding(
    registry: &FleetRegistry,
    root: &FleetSubnetRootEntry,
) -> FleetSubnetRootBinding {
    FleetSubnetRootBinding {
        authority: registry.authority.clone(),
        placement_subnet: root.placement_subnet,
        fleet_subnet_root: root.fleet_subnet_root,
        component_admissions: root.component_admissions.clone(),
        component_topology_digest: root.component_topology_digest,
        limits: root.limits.clone(),
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
