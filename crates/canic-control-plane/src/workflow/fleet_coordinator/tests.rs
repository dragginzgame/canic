//! Module: workflow::fleet_coordinator::tests
//!
//! Responsibility: qualify protected genesis commitment and canonical Coordinator queries.
//! Does not own: PocketIC installation or host effect-journal coverage.

use super::*;
use crate::storage::stable::fleet_admission::{
    FleetAdmissionAuthorityRecord, FleetAdmissionAuthorityStore,
    FleetAdmissionCoordinatorRootPhaseRecord, FleetAdmissionCoordinatorRootProgressRecord,
    FleetAdmissionMutationActionRecord, FleetAdmissionMutationOutcomeRecord,
    FleetAdmissionMutationRequestRecord, FleetAdmissionMutationResponseRecord,
    FleetAdmissionRetainedResultRecord,
};
use crate::storage::stable::fleet_coordinator::{
    FLEET_COORDINATOR_STATE_MAX_BYTES, FleetAdmissionPublicationActionRecord,
    FleetAdmissionPublicationRecord, FleetComponentDirectoryConfirmationIntentRecord,
    FleetComponentDirectoryConfirmationRecord, FleetComponentProvisioningStateRecord,
    FleetCoordinatorFundingStore, FleetCoordinatorRegistryData, FleetCoordinatorRegistryStore,
    FleetCoordinatorStateRecord,
};
use crate::view::fleet_coordinator::{
    FleetComponentDirectoryConfirmationCallView, FleetComponentDirectoryConfirmationDisposition,
    FleetComponentProvisioningRootAcceptanceDisposition,
    FleetComponentProvisioningRootProvisionCallView,
    FleetComponentProvisioningRootProvisionDisposition, FleetComponentRuntimeActivationDisposition,
    FleetRootFundingDisposition,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    task::{Context, Poll, Waker},
};

use canic_core::{
    bootstrap::parse_config_model,
    cdk::structures::storable::Storable,
    cdk::types::Cycles,
    control_plane_support::{
        config::ConfigModel,
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
            FleetComponentPublicationRootProgress, FleetSubnetRootProvisioningBatch,
            RootComponentActivationEvidence, RootComponentDirectorySynchronizationResponse,
            RootComponentProvisioningAcceptanceRequest, RootComponentProvisioningPhase,
            RootComponentProvisioningResult, RootComponentProvisioningStatusResponse,
            RootComponentPublicationEvidence, RootComponentPublicationRequest,
            RootProvisionedGroupMember, RootProvisionedGroupPlacement,
        },
        fleet_admission::{
            FleetAdmissionMutationAction, FleetAdmissionMutationOutcome,
            FleetAdmissionMutationRequest, FleetAdmissionPrepareRootStage,
            FleetAdmissionRootTransitionPhase,
        },
        fleet_funding::{
            FleetFundingPolicyRotationApplyRequest, FleetFundingPolicyRotationBeginRequest,
            FleetFundingPolicyRotationFundingSource, FleetFundingPolicyRotationPlacementEvidence,
            FleetFundingPolicyRotationPlan, FleetFundingPolicyRotationPlanHeader,
            FleetFundingPolicyRotationRootPlan, FleetFundingPolicyRotationRootReceipt,
            FleetFundingPolicyRotationStageRootRequest, FleetFundingPolicyUsage,
            FleetRootFundingAcceptanceReceipt, FleetRootFundingNoGrantReason,
            FleetRootFundingRequest, FleetRootFundingResponse,
        },
        fleet_registry::{
            FleetRegistryActivationRequest, FleetSubnetRootDeletionCompletionRequest,
            FleetSubnetRootDeletionExecutionRequest, FleetSubnetRootDeletionReadinessIntentRequest,
            FleetSubnetRootDeletionReadinessRequest, FleetSubnetRootDeletionStatusRequest,
            FleetSubnetRootDrainingPublicationRequest, FleetSubnetRootDrainingReservationRequest,
            FleetSubnetRootDrainingReservationStatusRequest, FleetSubnetRootEntry,
            FleetSubnetRootJoinRequest, FleetSubnetRootRemovalPublicationRequest,
            FleetSubnetRootStatus,
        },
        fleet_subnet_root::{
            FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES, FleetSubnetRootDrainingResponse,
            FleetSubnetRootFinalInventoryResponse,
        },
    },
    ids::{
        AppId, CanisterRole, CanonicalNetworkId, ComponentBinding, ComponentGroupPlacementId,
        ComponentInstanceId, ComponentSpecAdmission, CyclesFundingBudget, FleetAdmissionRule,
        FleetAdmissionSelector, FleetBinding, FleetCoordinatorBinding, FleetFundingProfile,
        FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetRootBinding, FleetSubnetRootLimits,
        FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    },
    shared_support::fleet_funding_policy::{
        coordinator_root_funding_policy_hash, fleet_funding_policy_rotation_operation_id,
        fleet_funding_policy_rotation_plan_digest, fleet_funding_policy_rotation_roots_digest,
        fleet_root_funding_operation_id, fleet_subnet_root_funding_policy_hash,
    },
    shared_support::{
        fleet_admission_authority::{
            FLEET_ADMISSION_AUTHORITY_SCHEMA_VERSION, FleetAdmissionMutationActionModel,
            FleetAdmissionMutationRequestModel, FleetAdmissionRootCatalogAuthorityModel,
            MAX_FLEET_ADMISSION_PUBLICATIONS, fleet_admission_mutation_request_digest,
        },
        fleet_admission_policy::{
            compile_installed_fleet_admission_policy, fleet_admission_participant_catalog_digest,
            fleet_admission_root_receipt_digest_from_binding,
        },
    },
};

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn poll_ready<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("future unexpectedly awaited an external effect"),
    }
}

#[test]
fn maximum_production_root_prepare_command_fits_update_envelope() {
    let fleet = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([0xe1; 32]),
        },
        app: AppId::from("a".repeat(40)),
    };
    let principals = (0_u16..256)
        .map(|index| {
            let mut bytes = [0xa5; 29];
            bytes[..2].copy_from_slice(&index.to_be_bytes());
            Principal::from_slice(&bytes)
        })
        .collect::<Vec<_>>();
    let rules = (0..32)
        .map(|index| FleetAdmissionRule {
            selector: FleetAdmissionSelector::ComponentSpec(
                format!("r{index:039}")
                    .parse()
                    .expect("maximum Component Spec ID"),
            ),
            principals: principals[(index * 4)..(index * 4 + 4)].to_vec(),
        })
        .collect::<Vec<_>>();
    let successor =
        compile_installed_fleet_admission_policy(fleet.clone(), u64::MAX, principals, rules)
            .expect("maximum successor policy");
    let command = RemoteRootCommand::PrepareFleetAdmission(FleetAdmissionPrepareRootRequest {
        authority: FleetCoordinatorBinding {
            fleet,
            coordinator_subnet: SubnetId::from_principal(principal(1)),
            coordinator: principal(2),
        },
        operation_id: [0xe2; 32],
        expected_generation: u64::MAX - 1,
        expected_policy_digest: [0xe3; 32],
        successor,
        stage: FleetAdmissionPrepareRootStage::Reserve,
    });

    let bytes = candid::encode_one(command).expect("production Root command Candid");
    eprintln!(
        "maximum production Root prepare command bytes: {}",
        bytes.len()
    );
    assert!(
        bytes.len() <= canic_core::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES,
        "maximum production Root command must fit the frozen 16 KiB envelope"
    );
}

#[test]
fn admission_publications_extend_canonical_registry_history_and_replay_exactly() {
    let config = scale_out_coordinator_config();
    drive_terminal_fresh_install(&config);
    let initial = FleetCoordinatorWorkflow::registry().expect("active Registry");
    let added = principal(201);
    let mut added_principals = initial.admission.fleet_principals.clone();
    added_principals.push(added);
    added_principals.sort_unstable();
    let added_policy = compile_installed_fleet_admission_policy(
        initial.admission.fleet.clone(),
        initial.admission.generation + 1,
        added_principals,
        initial.admission.rules.clone(),
    )
    .expect("added policy");
    let add = FleetAdmissionMutationRequestModel {
        authority: initial.authority.binding.clone(),
        expected_generation: initial.admission.generation,
        expected_policy_digest: initial.admission.policy_digest,
        action: FleetAdmissionMutationActionModel::Add,
        selector: FleetAdmissionSelector::Fleet,
        principal: added,
        operation_id: [202; 32],
        successor_policy_digest: added_policy.policy_digest,
        participant_catalog_digest: [204; 32],
        participant_count: 1,
    };

    let published = crate::ops::fleet_coordinator::FleetCoordinatorOps::publish_admission_policy(
        add.clone(),
        added_policy.clone(),
    )
    .expect("publish first admission generation");
    assert_eq!(published.admission, added_policy);
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::publish_admission_policy(
            add,
            published.admission.clone(),
        )
        .expect("replay first admission publication"),
        published
    );

    let mut removed_principals = published.admission.fleet_principals.clone();
    removed_principals.retain(|principal| *principal != added);
    let removed_policy = compile_installed_fleet_admission_policy(
        published.admission.fleet.clone(),
        published.admission.generation + 1,
        removed_principals,
        published.admission.rules.clone(),
    )
    .expect("removed policy");
    let remove = FleetAdmissionMutationRequestModel {
        authority: published.authority.binding.clone(),
        expected_generation: published.admission.generation,
        expected_policy_digest: published.admission.policy_digest,
        action: FleetAdmissionMutationActionModel::Remove,
        selector: FleetAdmissionSelector::Fleet,
        principal: added,
        operation_id: [203; 32],
        successor_policy_digest: removed_policy.policy_digest,
        participant_catalog_digest: [204; 32],
        participant_count: 1,
    };
    let removed = crate::ops::fleet_coordinator::FleetCoordinatorOps::publish_admission_policy(
        remove,
        removed_policy.clone(),
    )
    .expect("publish second admission generation");
    assert_eq!(removed.admission, removed_policy);
    assert_eq!(removed.revision, initial.revision + 2);

    let durable = FleetCoordinatorRegistryStore::export()
        .current
        .expect("Coordinator Registry state");
    assert_eq!(durable.admission_publications.len(), 2);
    assert_eq!(
        FleetCoordinatorWorkflow::registry().expect("reconstructed Registry head"),
        removed
    );
}

#[test]
fn maximum_admission_publication_history_fits_coordinator_registry_cell() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let coordinator = principal(205);
    FleetCoordinatorWorkflow::initialize(init_args(coordinator), principal(206), true, coordinator)
        .expect("initialize Coordinator Registry capacity fixture");
    let mut current = FleetCoordinatorRegistryStore::export()
        .current
        .expect("Coordinator Registry state");
    let maximum_authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([u8::MAX; 32]),
                },
                app: AppId::from("a".repeat(40)),
            },
            coordinator_subnet: SubnetId::from_principal(principal(207)),
            coordinator: principal(208),
        },
        epoch: u64::MAX,
    };
    let maximum_version = FleetRegistryVersion {
        authority: maximum_authority,
        revision: u64::MAX,
        content_hash: [u8::MAX; 32],
    };
    let selector = FleetAdmissionSelector::ComponentSpec(
        "a".repeat(40).parse().expect("maximum Component Spec ID"),
    );
    current.admission_publications = (0..MAX_FLEET_ADMISSION_PUBLICATIONS)
        .map(|index| {
            let operation = u32::try_from(index).expect("bounded publication index");
            let mut operation_id = [u8::MAX; 32];
            operation_id[..4].copy_from_slice(&operation.to_be_bytes());
            FleetAdmissionPublicationRecord {
                operation_id,
                action: FleetAdmissionPublicationActionRecord::Remove,
                selector: selector.clone(),
                principal: principal(u8::MAX),
                expected_generation: u64::MAX - 1,
                expected_policy_digest: [u8::MAX; 32],
                successor_generation: u64::MAX,
                successor_policy_digest: [u8::MAX; 32],
                previous_version: maximum_version.clone(),
                version: maximum_version.clone(),
            }
        })
        .collect();
    let state = FleetCoordinatorStateRecord {
        current: Some(current),
    };
    let encoded = state.to_bytes();

    eprintln!(
        "maximum admission publication history encoded bytes: {}",
        encoded.len()
    );
    assert!(encoded.len() <= FLEET_COORDINATOR_STATE_MAX_BYTES as usize);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "cross-domain terminal replay needs complete stable state"
)]
fn completed_admission_command_replays_before_cross_domain_collision_checks() {
    let config = scale_out_coordinator_config();
    drive_terminal_fresh_install(&config);
    let initial = FleetCoordinatorWorkflow::registry().expect("active Registry");
    let added = principal(204);
    let mut added_principals = initial.admission.fleet_principals.clone();
    added_principals.push(added);
    added_principals.sort_unstable();
    let added_policy = compile_installed_fleet_admission_policy(
        initial.admission.fleet.clone(),
        initial.admission.generation + 1,
        added_principals,
        initial.admission.rules.clone(),
    )
    .expect("added policy");
    let mut request = FleetAdmissionMutationRequestModel {
        authority: initial.authority.binding.clone(),
        expected_generation: initial.admission.generation,
        expected_policy_digest: initial.admission.policy_digest,
        action: FleetAdmissionMutationActionModel::Add,
        selector: FleetAdmissionSelector::Fleet,
        principal: added,
        operation_id: [205; 32],
        successor_policy_digest: added_policy.policy_digest,
        participant_catalog_digest: [204; 32],
        participant_count: 1,
    };
    let published = crate::ops::fleet_coordinator::FleetCoordinatorOps::publish_admission_policy(
        request.clone(),
        added_policy.clone(),
    )
    .expect("publish admission generation");
    let mut roots = published
        .fleet_subnet_roots
        .iter()
        .map(|root| FleetAdmissionCoordinatorRootProgressRecord {
            fleet_subnet_root: root.fleet_subnet_root,
            placement_subnet: root.placement_subnet,
            phase: FleetAdmissionCoordinatorRootPhaseRecord::Open,
            participant_catalog_digest: Some([206; 32]),
            participant_count: Some(1),
            last_receipt_hash: None,
        })
        .collect::<Vec<_>>();
    roots.sort_unstable_by(|left, right| {
        left.fleet_subnet_root
            .as_slice()
            .cmp(right.fleet_subnet_root.as_slice())
    });
    let catalogs = roots
        .iter()
        .map(|root| FleetAdmissionRootCatalogAuthorityModel {
            fleet_subnet_root: root.fleet_subnet_root,
            participant_catalog_digest: root
                .participant_catalog_digest
                .expect("participant catalog digest"),
            participant_count: root.participant_count.expect("participant count"),
        })
        .collect::<Vec<_>>();
    request.participant_catalog_digest = fleet_admission_participant_catalog_digest(&catalogs);
    request.participant_count = u32::try_from(catalogs.len()).expect("participant count");
    for root in &mut roots {
        root.last_receipt_hash = Some(fleet_admission_root_receipt_digest_from_binding(
            request.operation_id,
            FleetAdmissionRootTransitionPhase::Converged,
            &request.authority,
            root.placement_subnet,
            root.fleet_subnet_root,
            added_policy.generation,
            added_policy.policy_digest,
            root.participant_catalog_digest
                .expect("participant catalog digest"),
            root.participant_count.expect("participant count"),
        ));
    }
    assert!(FleetAdmissionAuthorityStore::replace(
        FleetAdmissionAuthorityRecord {
            schema_version: FLEET_ADMISSION_AUTHORITY_SCHEMA_VERSION,
            active_policy: added_policy.clone(),
            current_transition: None,
            last_result: Some(FleetAdmissionRetainedResultRecord {
                request: FleetAdmissionMutationRequestRecord {
                    authority: request.authority.clone(),
                    expected_generation: request.expected_generation,
                    expected_policy_digest: request.expected_policy_digest,
                    action: FleetAdmissionMutationActionRecord::Add,
                    selector: request.selector.clone(),
                    principal: request.principal,
                    operation_id: request.operation_id,
                    successor_policy_digest: request.successor_policy_digest,
                    participant_catalog_digest: request.participant_catalog_digest,
                    participant_count: request.participant_count,
                },
                request_hash: fleet_admission_mutation_request_digest(&request),
                response: FleetAdmissionMutationResponseRecord {
                    outcome: FleetAdmissionMutationOutcomeRecord::Converged,
                    operation_id: request.operation_id,
                    generation: added_policy.generation,
                    policy_digest: added_policy.policy_digest,
                },
                roots,
            }),
        }
    ));

    let replay = FleetCoordinatorWorkflow::mutate_admission(FleetAdmissionMutationRequest {
        authority: request.authority,
        expected_generation: request.expected_generation,
        expected_policy_digest: request.expected_policy_digest,
        action: FleetAdmissionMutationAction::Add,
        selector: request.selector,
        principal: request.principal,
        operation_id: request.operation_id,
        successor_policy_digest: request.successor_policy_digest,
        participant_catalog_digest: request.participant_catalog_digest,
        participant_count: request.participant_count,
    })
    .expect("replay completed admission command");
    assert_eq!(replay.outcome, FleetAdmissionMutationOutcome::Converged);
    assert_eq!(replay.generation, added_policy.generation);
    assert_eq!(replay.policy_digest, added_policy.policy_digest);
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

fn five_component_coordinator_config() -> ConfigModel {
    parse_config_model(
        &ORDINARY_SCALE_OUT_COORDINATOR_CONFIG
            .replace("maximum_instances = 3", "maximum_instances = 10")
            .replace(
                r#"[component_groups.project_cell.components.project]
component_spec = "projects""#,
                r#"[component_groups.project_cell.components.alpha]
component_spec = "projects"

[component_groups.project_cell.components.beta]
component_spec = "projects"

[component_groups.project_cell.components.delta]
component_spec = "projects"

[component_groups.project_cell.components.epsilon]
component_spec = "projects"

[component_groups.project_cell.components.gamma]
component_spec = "projects""#,
            )
            .replace(
                "placement.maximum_per_root = 1",
                "placement.maximum_per_root = 2",
            ),
    )
    .expect("valid five-Component Coordinator config")
}

fn repeated_scale_out_coordinator_config() -> ConfigModel {
    parse_config_model(
        &SCALE_OUT_COORDINATOR_CONFIG
            .replace("maximum_instances = 3", "maximum_instances = 6")
            .replace("maximum_placements = 2", "maximum_placements = 4")
            .replace(
                "placement.maximum_per_root = 1",
                "placement.maximum_per_root = 3",
            )
            .replace(
                "placement.maximum_members_per_root = 1",
                "placement.maximum_members_per_root = 3",
            ),
    )
    .expect("valid repeated scale-out Coordinator config")
}

fn packed_active_pool_coordinator_config() -> ConfigModel {
    parse_config_model(
        &SCALE_OUT_COORDINATOR_CONFIG
            .replace("maximum_instances = 3", "maximum_instances = 6")
            .replace("initial_placements = 1", "initial_placements = 2")
            .replace("maximum_placements = 2", "maximum_placements = 4")
            .replace(
                "placement.maximum_per_root = 1",
                "placement.maximum_per_root = 2",
            )
            .replace(
                "placement.minimum_distinct_roots = 1",
                "placement.minimum_distinct_roots = 2",
            )
            .replace(
                "placement.maximum_members_per_root = 1",
                "placement.maximum_members_per_root = 2",
            ),
    )
    .expect("valid packed ActivePool Coordinator config")
}

fn init_args(coordinator: Principal) -> FleetCoordinatorInitArgs {
    init_args_with_config(coordinator, &coordinator_config())
}

fn init_args_with_config(coordinator: Principal, config: &ConfigModel) -> FleetCoordinatorInitArgs {
    let component_deployment_configuration = config
        .compile_component_deployment_configuration()
        .expect("Component deployment configuration");
    let fleet = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([7; 32]),
        },
        app: AppId::from("demo"),
    };
    FleetCoordinatorInitArgs {
        configured_app: AppId::from("demo"),
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: fleet.clone(),
                coordinator_subnet: SubnetId::from_principal(principal(2)),
                coordinator,
            },
            epoch: 1,
        },
        admission: crate::test_support::fleet_admission_policy(fleet),
        root_funding: Some(crate::test_support::coordinator_root_funding_policy()),
        component_deployment_configuration,
    }
}

#[test]
fn protected_init_commits_exact_genesis_and_supports_exact_retry() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let coordinator = principal(3);
    let controller = principal(4);
    let args = init_args(coordinator);
    let expected_root_funding = args.root_funding.clone();

    let mut invalid_policy = args.clone();
    invalid_policy
        .root_funding
        .as_mut()
        .expect("Coordinator root-funding policy")
        .minimum_reserve_cycles = Cycles::new(1);
    let invalid =
        FleetCoordinatorWorkflow::initialize(invalid_policy, controller, true, coordinator)
            .expect_err("invalid Coordinator funding policy must reject before commitment");
    assert_eq!(
        invalid.public_error().code(),
        canic_core::diagnostics::codes::REQUEST_INVALID.raw_code()
    );
    assert!(FleetCoordinatorRegistryStore::export().current.is_none());

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
        unauthorized.public_error().code(),
        canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
    );

    let wrong_canister = FleetCoordinatorWorkflow::initialize(
        init_args(principal(6)),
        controller,
        true,
        coordinator,
    )
    .expect_err("reject wrong Coordinator binding");
    assert_eq!(
        wrong_canister.public_error().code(),
        canic_core::diagnostics::codes::REQUEST_INVALID.raw_code()
    );

    let durable = FleetCoordinatorRegistryStore::export();
    assert_eq!(
        durable
            .current
            .as_ref()
            .expect("Coordinator state")
            .root_funding,
        expected_root_funding
    );
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
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID
    );
    FleetCoordinatorRegistryStore::import(durable);
}

#[test]
fn rootless_coordinator_initializes_an_empty_disabled_capable_funding_ledger() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let coordinator = principal(8);
    let mut args = init_args(coordinator);
    args.root_funding = None;

    FleetCoordinatorWorkflow::initialize(args, principal(9), true, coordinator)
        .expect("initialize rootless Coordinator");
    let funding = FleetCoordinatorFundingStore::export()
        .current
        .expect("empty funding ledger");
    assert!(funding.funding_enabled);
    assert!(funding.fleet_window.is_none());
    assert!(funding.roots.is_empty());
    assert!(
        FleetCoordinatorOps::set_root_funding_enabled(false)
            .expect("disable empty funding ledger")
            .changed
    );
}

#[test]
fn coordinator_operation_status_resolves_the_durable_domain_from_one_id() {
    let (_config, plan_hash) = prepare_two_root_acceptance_plan();

    let status = FleetCoordinatorWorkflow::operation_status([101; 32])
        .expect("resolve active Component provisioning operation");
    let crate::dto::fleet_coordinator::CoordinatorOperationStatusResponse::ComponentProvisioning(
        status,
    ) = status
    else {
        panic!("expected Component provisioning status");
    };
    assert_eq!(status.operation_id, [101; 32]);
    assert_eq!(status.plan_hash, plan_hash);

    let Err(invalid) = FleetCoordinatorWorkflow::operation_status([0; 32]) else {
        panic!("zero operation ID must be rejected");
    };
    assert_eq!(
        invalid.public_error().code(),
        canic_core::diagnostics::codes::REQUEST_INVALID.raw_code()
    );
}

#[test]
fn coordinator_root_removal_status_exists_from_the_accepted_reservation() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let (root, _peer, version) = activate_two_roots(principal(102));
    let request = root_draining_reservation_request(&root, &version, [103; 32]);
    let reservation = FleetCoordinatorWorkflow::prepare_root_draining_reservation(request)
        .expect("accept root-removal reservation");

    let status = FleetCoordinatorWorkflow::operation_status([103; 32])
        .expect("resolve accepted root-removal operation");
    let crate::dto::fleet_coordinator::CoordinatorOperationStatusResponse::RootRemoval(status) =
        status
    else {
        panic!("expected Root removal status");
    };
    assert_eq!(status.operation_id, [103; 32]);
    assert_eq!(status.reservation, reservation);
    assert_eq!(status.readiness_intent, None);
    assert_eq!(status.readiness, None);
    assert_eq!(status.execution, None);
    assert_eq!(status.completion, None);
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
        stale.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
    );

    let mut conflicting_entry = first_entry;
    conflicting_entry.limits.maximum_registry_bytes += 1;
    let conflict = FleetCoordinatorWorkflow::join_root(FleetSubnetRootJoinRequest {
        expected_registry: second.version,
        entry: conflicting_entry,
    })
    .expect_err("an existing root identity cannot change authority");
    assert_eq!(
        conflict.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
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
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID.raw_code()
    );
}

#[test]
fn root_join_rejects_a_fleet_budget_that_cannot_admit_the_root_target() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let coordinator = principal(17);
    let mut args = init_args(coordinator);
    args.root_funding
        .as_mut()
        .expect("Coordinator root-funding policy")
        .budget
        .maximum_cycles = Cycles::new(50_000_000_000);
    let topology = args
        .component_deployment_configuration
        .component_topology
        .clone();
    FleetCoordinatorWorkflow::initialize(args, principal(18), true, coordinator)
        .expect("commit bounded genesis");
    let genesis = FleetCoordinatorWorkflow::version().expect("genesis version");

    let rejected = FleetCoordinatorWorkflow::join_root(FleetSubnetRootJoinRequest {
        expected_registry: genesis,
        entry: joining_entry(&topology, 7, 19, 1),
    })
    .expect_err("Fleet budget below the root target must reject");
    assert_eq!(
        rejected.public_error().code(),
        canic_core::diagnostics::codes::STATE_INVALID.raw_code()
    );
    assert!(
        FleetCoordinatorWorkflow::registry()
            .expect("unchanged Registry")
            .fleet_subnet_roots
            .is_empty()
    );
}

#[test]
fn root_draining_reservation_is_durable_hash_bound_and_target_readable() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let coordinator = principal(70);
    let (first, second, active_version) = activate_two_roots(coordinator);
    let request = root_draining_reservation_request(&first, &active_version, [71; 32]);
    let response =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::prepare_root_draining_reservation(
            request.clone(),
            72,
        )
        .expect("prepare root-draining reservation");
    assert_eq!(response.request, request);
    assert_eq!(response.coordinator, coordinator);
    assert_eq!(response.prepared_at_ns, 72);
    assert_ne!(response.reservation_hash, [0; 32]);

    let status_request = FleetSubnetRootDrainingReservationStatusRequest {
        operation_id: [71; 32],
        fleet_subnet_root: first.fleet_subnet_root,
    };
    assert_eq!(
        FleetCoordinatorWorkflow::root_draining_reservation_status(
            first.fleet_subnet_root,
            false,
            status_request,
        )
        .expect("target root reads its reservation"),
        response
    );
    assert_eq!(
        FleetCoordinatorWorkflow::root_draining_reservation_status(
            principal(73),
            true,
            status_request,
        )
        .expect("controller reads reservation"),
        response
    );
    let forbidden = FleetCoordinatorWorkflow::root_draining_reservation_status(
        principal(74),
        false,
        status_request,
    )
    .expect_err("foreign caller cannot read reservation");
    assert_eq!(
        forbidden.public_error().code(),
        canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
    );

    FleetCoordinatorWorkflow::publish_root_draining(root_draining_publication_request(
        &second,
        &active_version,
        [75; 32],
    ))
    .expect("advance an unrelated root through a later Registry revision");
    let durable = FleetCoordinatorRegistryStore::export();
    FleetCoordinatorRegistryStore::import(durable);
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::prepare_root_draining_reservation(
            request, 999
        )
        .expect("exact reservation retry after restart"),
        response
    );
    let publication_registry =
        FleetCoordinatorWorkflow::version().expect("Registry version after unrelated root drains");
    let mut publication = root_draining_publication_request(&first, &active_version, [71; 32]);
    publication.expected_registry = publication_registry.clone();
    let published = FleetCoordinatorWorkflow::publish_root_draining(publication)
        .expect("consume earlier reservation after unrelated Registry revision");
    assert_eq!(published.previous_version, publication_registry);
    assert_eq!(published.root_draining.active_registry, active_version);
    let durable = FleetCoordinatorRegistryStore::export();
    let mut corrupted = durable.clone();
    corrupted
        .current
        .as_mut()
        .expect("Coordinator state")
        .root_draining_reservations[0]
        .response
        .reservation_hash[0] ^= 1;
    FleetCoordinatorRegistryStore::import(corrupted);
    let invalid = FleetCoordinatorWorkflow::registry()
        .expect_err("corrupt reservation hash must fail closed");
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID
    );
    FleetCoordinatorRegistryStore::import(durable);
}

#[test]
fn root_draining_reservation_rejects_stale_and_reused_authority() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let (first, second, active_version) = activate_two_roots(principal(75));
    let request = root_draining_reservation_request(&first, &active_version, [76; 32]);
    let before = FleetCoordinatorRegistryStore::export();

    let mut zero = request.clone();
    zero.operation_id = [0; 32];
    let invalid =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::prepare_root_draining_reservation(
            zero, 77,
        )
        .expect_err("zero reservation operation rejects");
    assert_eq!(
        invalid.public_error().code(),
        canic_core::diagnostics::codes::REQUEST_INVALID.raw_code()
    );

    let mut stale = request.clone();
    stale.expected_registry.content_hash[0] ^= 1;
    let conflict =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::prepare_root_draining_reservation(
            stale, 77,
        )
        .expect_err("stale Registry hash rejects");
    assert_eq!(
        conflict.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), before);

    crate::ops::fleet_coordinator::FleetCoordinatorOps::prepare_root_draining_reservation(
        request.clone(),
        77,
    )
    .expect("prepare exact reservation");
    let mut reused_root = request;
    reused_root.operation_id = [78; 32];
    assert_reservation_conflict(reused_root, "one root cannot reserve twice");
    let reused_operation = root_draining_reservation_request(&second, &active_version, [76; 32]);
    assert_reservation_conflict(reused_operation, "operation cannot name another root");

    let wrong_status = FleetSubnetRootDrainingReservationStatusRequest {
        operation_id: [76; 32],
        fleet_subnet_root: second.fleet_subnet_root,
    };
    let conflict = FleetCoordinatorWorkflow::root_draining_reservation_status(
        second.fleet_subnet_root,
        false,
        wrong_status,
    )
    .expect_err("status cannot substitute another root");
    assert_eq!(
        conflict.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
}

#[test]
fn component_plan_and_root_draining_reservation_have_one_atomic_winner() {
    let config = coordinator_config();
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let (first, _, active_version) = activate_two_roots(principal(79));
    let registry = FleetCoordinatorWorkflow::registry().expect("active Registry");
    let plan = fresh_component_plan(&config, &registry);
    let reservation = root_draining_reservation_request(&first, &active_version, [80; 32]);
    crate::ops::fleet_coordinator::FleetCoordinatorOps::prepare_root_draining_reservation(
        reservation,
        81,
    )
    .expect("reservation wins first");
    let before_plan = FleetCoordinatorRegistryStore::export();
    let conflict = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(
            &config,
            FleetComponentProvisioningPrepareRequest {
                operation_id: [82; 32],
                plan,
            },
            83,
        )
        .expect_err("plan cannot select reserved root");
    assert_eq!(
        conflict.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), before_plan);

    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let (first, _, active_version) = activate_two_roots(principal(84));
    let registry = FleetCoordinatorWorkflow::registry().expect("active Registry");
    let plan = fresh_component_plan(&config, &registry);
    crate::ops::fleet_coordinator::FleetCoordinatorOps::prepare_component_provisioning_for_test(
        &config,
        FleetComponentProvisioningPrepareRequest {
            operation_id: [85; 32],
            plan,
        },
        86,
    )
    .expect("plan wins first");
    let before_reservation = FleetCoordinatorRegistryStore::export();
    let reservation = root_draining_reservation_request(&first, &active_version, [87; 32]);
    assert_reservation_conflict(reservation, "reservation cannot overtake durable plan");
    assert_eq!(FleetCoordinatorRegistryStore::export(), before_reservation);
}

#[test]
fn scale_out_cannot_select_a_root_reserved_after_fresh_provisioning() {
    let config = scale_out_coordinator_config();
    drive_terminal_fresh_install(&config);
    let registry = FleetCoordinatorWorkflow::registry().expect("published Fleet Registry");
    let target = registry
        .fleet_subnet_roots
        .get(1)
        .expect("unreferenced second root");
    let version = FleetRegistryOps::version(
        &registry.authority,
        &config
            .compile_component_topology()
            .expect("Component Topology"),
        &registry,
    )
    .expect("published Registry version");
    let reservation = root_draining_reservation_request(target, &version, [89; 32]);
    crate::ops::fleet_coordinator::FleetCoordinatorOps::prepare_root_draining_reservation(
        reservation,
        900,
    )
    .expect("reserve unreferenced root after fresh provisioning");
    let before = FleetCoordinatorRegistryStore::export();
    let scale_out = one_placement_scale_out_plan(&config, &registry);
    let conflict = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(
            &config,
            FleetComponentProvisioningPrepareRequest {
                operation_id: [90; 32],
                plan: scale_out,
            },
            901,
        )
        .expect_err("scale-out cannot select reserved root");
    assert_eq!(
        conflict.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), before);
}

fn root_draining_reservation_request(
    root: &FleetSubnetRootEntry,
    registry: &FleetRegistryVersion,
    operation_id: [u8; 32],
) -> FleetSubnetRootDrainingReservationRequest {
    let mut expected_root = root.clone();
    expected_root.status = FleetSubnetRootStatus::Active;
    FleetSubnetRootDrainingReservationRequest {
        operation_id,
        expected_registry: registry.clone(),
        expected_root,
    }
}

fn root_draining_publication_request(
    root: &FleetSubnetRootEntry,
    registry: &FleetRegistryVersion,
    operation_id: [u8; 32],
) -> FleetSubnetRootDrainingPublicationRequest {
    let reservation =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::prepare_root_draining_reservation(
            root_draining_reservation_request(root, registry, operation_id),
            1,
        )
        .expect("prepare root-draining reservation for publication");
    root_draining_publication_request_with_hash(
        root,
        registry,
        registry,
        operation_id,
        reservation.reservation_hash,
    )
}

fn root_draining_publication_request_with_hash(
    root: &FleetSubnetRootEntry,
    source_registry: &FleetRegistryVersion,
    publication_registry: &FleetRegistryVersion,
    operation_id: [u8; 32],
    reservation_hash: [u8; 32],
) -> FleetSubnetRootDrainingPublicationRequest {
    FleetSubnetRootDrainingPublicationRequest {
        expected_registry: publication_registry.clone(),
        root_draining: FleetSubnetRootDrainingResponse {
            operation_id,
            fleet_subnet_root: root.fleet_subnet_root,
            placement_subnet: root.placement_subnet,
            active_registry: source_registry.clone(),
            reservation_hash,
            component_topology_digest: root.component_topology_digest,
            active_release_set: root.active_release_set,
            next_allocation_sequence: 1,
            reserved_component_instances: 0,
            committed_component_instances: 0,
            managed_descendants: 0,
            known_created_component_canisters: 0,
            root_registry_encoded_bytes: 0,
            started_at_ns: 1,
        },
    }
}

#[test]
fn root_draining_publication_requires_one_exact_retained_reservation() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let (root, _, version) = activate_two_roots(principal(91));
    let missing =
        root_draining_publication_request_with_hash(&root, &version, &version, [92; 32], [93; 32]);
    let unavailable = FleetCoordinatorWorkflow::publish_root_draining(missing)
        .expect_err("publication without retained reservation must fail closed");
    assert_eq!(
        unavailable.public_error().code(),
        canic_core::diagnostics::codes::STATE_UNAVAILABLE.raw_code()
    );

    let reservation =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::prepare_root_draining_reservation(
            root_draining_reservation_request(&root, &version, [92; 32]),
            94,
        )
        .expect("prepare exact reservation");
    let wrong_hash =
        root_draining_publication_request_with_hash(&root, &version, &version, [92; 32], {
            let mut hash = reservation.reservation_hash;
            hash[0] ^= 1;
            hash
        });
    let conflict = FleetCoordinatorWorkflow::publish_root_draining(wrong_hash)
        .expect_err("publication with substituted reservation hash must fail closed");
    assert_eq!(
        conflict.public_error().code(),
        canic_core::diagnostics::codes::REQUEST_INVALID.raw_code()
    );
}

fn assert_reservation_conflict(
    request: FleetSubnetRootDrainingReservationRequest,
    message: &'static str,
) {
    let conflict =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::prepare_root_draining_reservation(
            request, 88,
        )
        .expect_err(message);
    assert_eq!(
        conflict.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
}

#[test]
fn initial_service_publication_commits_registry_receipt_and_phase_atomically() {
    let (config, plan_hash) = prepare_two_root_acceptance_plan();
    let provisioned = drive_components_provisioned(&config, plan_hash);
    let request = root_provision_advance_request(&provisioned);
    let before = FleetCoordinatorRegistryStore::export();
    let source_version = provisioned.fleet_registry.clone();

    assert_service_publication_cursor(&config, &provisioned, &request, &before);
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
        advance_component_provisioning_root_acceptance_for_test(&config, &request, 162)
        .expect("completed root acceptance remains observational");
    assert!(matches!(
        replay,
        FleetComponentProvisioningRootAcceptanceDisposition::Current(_)
    ));
    assert_eq!(FleetCoordinatorRegistryStore::export(), before_replay);
}

#[test]
fn pending_root_retry_failure_is_typed_bounded_and_restart_safe() {
    let (config, plan_hash) = prepare_two_root_acceptance_plan();
    let provisioned = drive_components_provisioned(&config, plan_hash);
    let service_request = root_provision_advance_request(&provisioned);
    let (published, _) = commit_initial_service_publication(
        &service_request,
        provisioned.fleet_registry,
        provisioned.components_provisioned_at_ns,
    );
    let request = root_provision_advance_request(&published);
    crate::ops::fleet_coordinator::FleetCoordinatorOps::advance_component_directory_confirmation(
        &request, 161,
    )
    .expect("persist Directory confirmation intent");
    let status_request = FleetComponentProvisioningStatusRequest {
        operation_id: published.operation_id,
        plan_hash,
    };
    let diagnostic_code = canic_core::diagnostics::codes::STATE_UNAVAILABLE
        .raw_code()
        .raw();
    let failed = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_provisioning_root_failure(status_request, diagnostic_code, 162)
        .expect("retain exact Root retry failure");
    let failure = failed
        .pending_root_failure
        .expect("pending retry exposes its typed Root failure");
    assert_eq!(
        failure.fleet_subnet_root,
        failed.publication_in_flight_root.unwrap()
    );
    assert_eq!(
        failure.stage,
        canic_core::dto::component_provisioning::FleetComponentProvisioningRetryStage::DirectoryConfirmation
    );
    assert_eq!(failure.diagnostic_code, diagnostic_code);
    assert_eq!(failure.failed_at_ns, 162);

    let durable = FleetCoordinatorRegistryStore::export();
    FleetCoordinatorRegistryStore::import(durable);
    let restored =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::component_provisioning_status_for_test(
            &config,
            status_request,
        )
        .expect("restore pending Root retry diagnostic");
    assert_eq!(restored.pending_root_failure, Some(failure));
}

fn assert_service_publication_cursor(
    config: &ConfigModel,
    provisioned: &FleetComponentProvisioningStatusResponse,
    request: &FleetComponentProvisioningAdvanceRequest,
    before: &FleetCoordinatorRegistryData,
) {
    let mut completed_root_retry = *request;
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
            advance_component_provisioning_root_for_test(config, request, 160)
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
        invalid.public_error().code(),
        canic_core::diagnostics::codes::REQUEST_INVALID.raw_code()
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
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID
    );

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
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID
    );

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
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID.raw_code()
    );
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
fn grouped_root_lifecycle_fence_is_exact_to_referenced_root() {
    let config = scale_out_coordinator_config();
    drive_terminal_fresh_install(&config);
    let registry = FleetCoordinatorWorkflow::registry().expect("terminal Fleet Registry");
    let current = FleetCoordinatorRegistryStore::export()
        .current
        .expect("Coordinator state");
    let occupied_root = current.component_group_deployments[0].placements[0].fleet_subnet_root;
    let empty_root = registry
        .fleet_subnet_roots
        .iter()
        .map(|root| root.fleet_subnet_root)
        .find(|root| *root != occupied_root)
        .expect("unreferenced Fleet Subnet Root");

    crate::ops::fleet_coordinator::FleetCoordinatorOps::require_root_lifecycle_open_for_test(
        &config,
        occupied_root,
    )
    .expect_err("committed placement and service authority must fence its root");
    crate::ops::fleet_coordinator::FleetCoordinatorOps::require_root_lifecycle_open_for_test(
        &config, empty_root,
    )
    .expect("another root without grouped authority remains lifecycle-open");

    let plan = one_placement_scale_out_plan(&config, &registry);
    let selected_root = plan.batches[0].root.fleet_subnet_root;
    assert_eq!(selected_root, empty_root);
    crate::ops::fleet_coordinator::FleetCoordinatorOps::prepare_component_provisioning_for_test(
        &config,
        FleetComponentProvisioningPrepareRequest {
            operation_id: [200; 32],
            plan,
        },
        1_000,
    )
    .expect("persist scale-out operation journal");
    crate::ops::fleet_coordinator::FleetCoordinatorOps::require_root_lifecycle_open_for_test(
        &config,
        selected_root,
    )
    .expect_err("in-progress grouped placement authority must fence its selected root");
}

#[test]
fn ordinary_group_placement_ledger_fences_its_root_without_a_service_binding() {
    let config = ordinary_scale_out_coordinator_config();
    drive_terminal_fresh_install(&config);
    let current = FleetCoordinatorRegistryStore::export()
        .current
        .expect("Coordinator state");
    assert!(current.registry.services.is_empty());
    let occupied_root = current.component_group_deployments[0].placements[0].fleet_subnet_root;

    crate::ops::fleet_coordinator::FleetCoordinatorOps::require_root_lifecycle_open_for_test(
        &config,
        occupied_root,
    )
    .expect_err("ordinary grouped placement authority must fence its exact root");
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
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID
    );
}

#[test]
fn active_pool_scale_out_and_restore_preserve_cross_document_authority() {
    let config = packed_active_pool_coordinator_config();
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    activate_two_roots_with_config_and_admission(principal(200), &config, 3);
    let initial_registry = FleetCoordinatorWorkflow::registry().expect("active Registry");
    let initial_plan = fresh_component_plan(&config, &initial_registry);
    let mut fresh = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(
            &config,
            FleetComponentProvisioningPrepareRequest {
                operation_id: [101; 32],
                plan: initial_plan,
            },
            100,
        )
        .expect("prepare two-root ActivePool plan");
    fresh = drive_root_acceptance(&config, fresh, 110);
    fresh = drive_root_provisioning(&config, fresh, 120);
    fresh = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        publish_component_provisioning_services(&root_provision_advance_request(&fresh), 200)
        .expect("publish initial ActivePool");
    fresh = drive_directory_confirmation(fresh, 210);
    fresh = drive_runtime_activation(fresh, 300);
    assert_eq!(
        fresh.phase,
        FleetComponentProvisioningPhase::RuntimesActivated
    );

    let source_registry = FleetCoordinatorWorkflow::registry().expect("initial ActivePool");
    let source_version = FleetCoordinatorWorkflow::version().expect("initial Registry version");
    assert_eq!(source_registry.services[0].members.len(), 2);
    let selected_root = source_registry.fleet_subnet_roots[0].fleet_subnet_root;
    let plan = scale_out_plan_on_root(&config, &source_registry, 2, 3, 0);
    let request = FleetComponentProvisioningPrepareRequest {
        operation_id: [203; 32],
        plan,
    };
    let terminal = drive_terminal_scale_out(&config, request.clone(), 1_000);
    assert_eq!(
        terminal.phase,
        FleetComponentProvisioningPhase::RuntimesActivated
    );

    let registry = FleetCoordinatorWorkflow::registry().expect("scaled ActivePool");
    let version = FleetCoordinatorWorkflow::version().expect("scaled Registry version");
    assert_eq!(version.revision, source_version.revision + 1);
    assert_eq!(registry.services[0].members.len(), 3);
    let selected_root_members = registry.services[0]
        .members
        .iter()
        .filter(|member| member.fleet_subnet_root == selected_root)
        .count();
    assert_eq!(selected_root_members, 2);
    assert_eq!(
        registry.services[0]
            .members
            .iter()
            .map(|member| member.fleet_subnet_root)
            .collect::<BTreeSet<_>>()
            .len(),
        2
    );

    let durable = FleetCoordinatorRegistryStore::export();
    let current = durable.current.as_ref().expect("Coordinator state");
    assert_eq!(current.service_publication_receipts.len(), 2);
    assert_eq!(
        current.component_group_deployments[0]
            .placements
            .iter()
            .filter(|placement| placement.fleet_subnet_root == selected_root)
            .count(),
        2
    );
    assert_restored_service_and_placement_authority(
        &config, &request, &terminal, &registry, &durable,
    );
    assert_packed_root_limit_rejects_without_mutation(&config, &registry, &durable);
}

fn assert_restored_service_and_placement_authority(
    config: &ConfigModel,
    request: &FleetComponentProvisioningPrepareRequest,
    terminal: &FleetComponentProvisioningStatusResponse,
    expected_registry: &FleetRegistry,
    durable: &FleetCoordinatorRegistryData,
) {
    let expected_deployments = durable
        .current
        .as_ref()
        .expect("Coordinator state")
        .component_group_deployments
        .clone();
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    FleetCoordinatorRegistryStore::import(durable.clone());

    assert_eq!(
        FleetCoordinatorWorkflow::registry().expect("restored Registry"),
        *expected_registry
    );
    let restored = FleetCoordinatorRegistryStore::export();
    assert_eq!(
        restored
            .current
            .as_ref()
            .expect("restored Coordinator state")
            .component_group_deployments,
        expected_deployments
    );
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::component_provisioning_status_for_test(
            config,
            FleetComponentProvisioningStatusRequest {
                operation_id: request.operation_id,
                plan_hash: terminal.plan_hash,
            },
        )
        .expect("replay terminal packed ActivePool scale-out after restore"),
        *terminal
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), *durable);

    let mut invalid_ordinal = durable.clone();
    invalid_ordinal
        .current
        .as_mut()
        .expect("Coordinator state")
        .component_group_deployments[0]
        .next_placement_ordinal += 1;
    FleetCoordinatorRegistryStore::import(invalid_ordinal.clone());
    let invalid = FleetCoordinatorWorkflow::registry()
        .expect_err("restored next ordinal cannot diverge from placement receipts");
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), invalid_ordinal);

    let mut invalid_service = durable.clone();
    invalid_service
        .current
        .as_mut()
        .expect("Coordinator state")
        .registry
        .services[0]
        .members[0]
        .group_placement
        .ordinal += 1;
    FleetCoordinatorRegistryStore::import(invalid_service.clone());
    let invalid = FleetCoordinatorWorkflow::registry()
        .expect_err("restored service member cannot diverge from publication authority");
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), invalid_service);

    FleetCoordinatorRegistryStore::import(durable.clone());
}

fn assert_packed_root_limit_rejects_without_mutation(
    config: &ConfigModel,
    registry: &FleetRegistry,
    durable: &FleetCoordinatorRegistryData,
) {
    let invalid_plan = scale_out_plan_on_root(config, registry, 3, 4, 0);
    let invalid = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(
            config,
            FleetComponentProvisioningPrepareRequest {
                operation_id: [204; 32],
                plan: invalid_plan,
            },
            2_000,
        )
        .expect_err("a third placement cannot enter the already-full root");
    assert_eq!(invalid.code(), canic_core::diagnostics::codes::STATE_FAILED);
    assert_eq!(FleetCoordinatorRegistryStore::export(), *durable);
}

#[test]
fn scale_out_confirms_affected_and_selected_roots_before_runtime_activation() {
    let config = scale_out_coordinator_config();
    drive_terminal_fresh_install(&config);
    let source_registry = FleetCoordinatorWorkflow::registry().expect("published Registry");
    let plan = one_placement_scale_out_plan(&config, &source_registry);
    assert_eq!(plan.batches.len(), 1);
    assert_eq!(plan.directory_confirmation_roots.len(), 2);
    let selected_root = plan.batches[0].root.fleet_subnet_root;
    let affected_only_root = plan
        .directory_confirmation_roots
        .iter()
        .copied()
        .find(|root| *root != selected_root)
        .expect("affected existing service-member root");

    let mut status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(
            &config,
            FleetComponentProvisioningPrepareRequest {
                operation_id: [202; 32],
                plan,
            },
            1_000,
        )
        .expect("prepare exact scale-out plan");
    status = drive_root_acceptance(&config, status, 1_010);
    status = drive_root_provisioning(&config, status, 1_020);
    status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        publish_component_provisioning_services(
            &root_provision_advance_request(&status),
            1_100,
        )
        .expect("publish scale-out service topology");

    let first_request = root_provision_advance_request(&status);
    let first_call = expect_scale_out_synchronization_call(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_directory_confirmation(&first_request, 1_110)
            .expect("persist affected-root synchronization intent"),
    );
    assert_eq!(first_call.0, affected_only_root);
    let durable_intent = FleetCoordinatorRegistryStore::export();
    FleetCoordinatorRegistryStore::import(durable_intent.clone());
    let replay = expect_scale_out_synchronization_call(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_directory_confirmation(&first_request, 9_999)
            .expect("reconcile affected-root synchronization intent"),
    );
    assert_eq!(replay, first_call);
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable_intent);

    let first_response = terminal_scale_out_synchronization_response(&first_call, 1, 1_111);
    status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_scale_out_directory_synchronization(
            &first_request,
            first_response,
            1_112,
        )
        .expect("record affected-root synchronization");
    assert_eq!(status.directory_confirmed_root_count, 1);
    assert!(status.current_publication.is_none());

    status = confirm_selected_scale_out_root(status, selected_root, 1_120);

    assert_eq!(
        status.phase,
        FleetComponentProvisioningPhase::DirectoriesConfirmed
    );
    assert_eq!(status.directory_confirmed_root_count, 2);
    assert_eq!(status.runtime_activated_root_count, 0);
    assert!(status.current_activation.is_none());
    let durable = FleetCoordinatorRegistryStore::export();
    FleetCoordinatorRegistryStore::import(durable.clone());
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::component_provisioning_status_for_test(
            &config,
            FleetComponentProvisioningStatusRequest {
                operation_id: status.operation_id,
                plan_hash: status.plan_hash,
            },
        )
        .expect("replay terminal Directory barrier after restart"),
        status
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable);

    status = activate_selected_scale_out_root(status, selected_root, affected_only_root, 1_140);
    assert_eq!(
        status.phase,
        FleetComponentProvisioningPhase::RuntimesActivated
    );
    assert_eq!(status.runtime_activated_root_count, 1);
    assert!(status.current_activation.is_none());

    assert_terminal_scale_out_placement(&status, selected_root);
}

#[test]
fn ordinary_scale_out_records_publication_without_registry_mutation() {
    let config = ordinary_scale_out_coordinator_config();
    drive_terminal_fresh_install(&config);
    let source_registry = FleetCoordinatorWorkflow::registry().expect("published Registry");
    let source_version = FleetCoordinatorWorkflow::version().expect("published version");
    assert!(source_registry.services.is_empty());

    let plan = one_placement_scale_out_plan(&config, &source_registry);
    let selected_root = plan.batches[0].root.fleet_subnet_root;
    assert_eq!(plan.directory_confirmation_roots, vec![selected_root]);
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
    let mut published = crate::ops::fleet_coordinator::FleetCoordinatorOps::
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

    published = confirm_selected_scale_out_root(published, selected_root, 1_110);
    assert_eq!(
        published.phase,
        FleetComponentProvisioningPhase::DirectoriesConfirmed
    );
    assert_eq!(published.directory_confirmed_root_count, 1);

    published = drive_runtime_activation(published, 1_200);
    assert_eq!(
        published.phase,
        FleetComponentProvisioningPhase::RuntimesActivated
    );
    assert_eq!(published.runtime_activated_root_count, 1);
    let terminal = FleetCoordinatorRegistryStore::export();
    let current = terminal.current.as_ref().expect("Coordinator state");
    assert_eq!(current.registry, source_registry);
    assert_eq!(current.component_group_deployments[0].placements.len(), 2);
}

#[test]
fn terminal_scale_out_rolls_into_compact_history_before_the_next_increase() {
    let scenario = prepare_repeated_scale_out();
    assert_retired_scale_out_replays(&scenario);
    complete_repeated_scale_out(scenario);
}

struct RepeatedScaleOutScenario {
    config: ConfigModel,
    first_request: FleetComponentProvisioningPrepareRequest,
    first: FleetComponentProvisioningStatusResponse,
    second: FleetComponentProvisioningStatusResponse,
    rolled: FleetCoordinatorRegistryData,
}

fn prepare_repeated_scale_out() -> RepeatedScaleOutScenario {
    let config = repeated_scale_out_coordinator_config();
    drive_terminal_fresh_install_with_admission(&config, 3);
    let registry = FleetCoordinatorWorkflow::registry().expect("published Registry");
    let first_request = FleetComponentProvisioningPrepareRequest {
        operation_id: [201; 32],
        plan: scale_out_plan(&config, &registry, 1, 2),
    };
    let first = drive_terminal_scale_out(&config, first_request.clone(), 1_000);
    assert_eq!(
        first.phase,
        FleetComponentProvisioningPhase::RuntimesActivated
    );
    assert_current_terminal_operation_rejects_conflicting_plan(&config, &first_request);

    let second_registry = FleetCoordinatorWorkflow::registry().expect("first scale-out Registry");
    assert_eq!(second_registry.services[0].members.len(), 2);
    let second_request = FleetComponentProvisioningPrepareRequest {
        operation_id: [203; 32],
        plan: scale_out_plan(&config, &second_registry, 2, 3),
    };
    assert_next_scale_out_rejects_time_regression(&config, &second_request, &first);
    let second = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(&config, second_request, 2_000)
        .expect("retire first terminal journal and prepare second increase");
    assert_eq!(second.phase, FleetComponentProvisioningPhase::Planned);
    let rolled = FleetCoordinatorRegistryStore::export();
    let current = rolled.current.as_ref().expect("Coordinator state");
    assert_eq!(current.component_scale_out_receipts.len(), 1);
    assert_eq!(current.registry.services[0].members.len(), 2);
    assert_eq!(
        current.component_scale_out_receipts[0].operation_id,
        first.operation_id
    );
    assert_eq!(
        current
            .component_scale_out
            .as_ref()
            .expect("second active scale-out")
            .operation_id,
        second.operation_id
    );
    assert_eq!(current.component_group_deployments[0].placements.len(), 2);
    assert_eq!(
        current.component_group_deployments[0].next_placement_ordinal,
        3
    );
    RepeatedScaleOutScenario {
        config,
        first_request,
        first,
        second,
        rolled,
    }
}

fn assert_retired_scale_out_replays(scenario: &RepeatedScaleOutScenario) {
    let RepeatedScaleOutScenario {
        config,
        first_request,
        first,
        rolled,
        ..
    } = scenario;
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::component_provisioning_status_for_test(
            config,
            FleetComponentProvisioningStatusRequest {
                operation_id: first.operation_id,
                plan_hash: first.plan_hash,
            },
        )
        .expect("retired operation status replay"),
        first.clone()
    );
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            prepare_component_provisioning_for_test(config, first_request.clone(), 9_999)
            .expect("retired exact preparation replay"),
        *first
    );
    assert_eq!(
        poll_ready(FleetCoordinatorWorkflow::advance_component_provisioning(
            &root_provision_advance_request(first),
        ))
        .expect("retired terminal advance replay"),
        *first
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), *rolled);

    let mut conflicting_retry = first_request.clone();
    conflicting_retry.plan.operation = FleetComponentProvisioningOperation::ScaleOut {
        deployment: "project_cells".parse().expect("deployment ID"),
        previous_placements: 1,
        requested_placements: 3,
    };
    let conflict = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(config, conflicting_retry, 10_000)
        .expect_err("retired operation cannot select different plan authority");
    assert_eq!(
        conflict.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), *rolled);
}

fn complete_repeated_scale_out(scenario: RepeatedScaleOutScenario) {
    let RepeatedScaleOutScenario {
        config,
        second,
        rolled,
        ..
    } = scenario;
    FleetCoordinatorRegistryStore::import(rolled);
    let second = drive_prepared_scale_out(&config, second, 2_010);
    assert_eq!(
        second.phase,
        FleetComponentProvisioningPhase::RuntimesActivated
    );
    let terminal = FleetCoordinatorRegistryStore::export();
    let current = terminal.current.as_ref().expect("Coordinator state");
    assert_eq!(current.component_scale_out_receipts.len(), 1);
    assert_eq!(current.registry.services[0].members.len(), 3);
    let placements = &current.component_group_deployments[0].placements;
    assert_eq!(
        placements
            .iter()
            .map(|placement| (placement.placement.ordinal, placement.operation_id))
            .collect::<Vec<_>>(),
        vec![(0, [101; 32]), (1, [201; 32]), (2, [203; 32])]
    );

    let mut corrupted = terminal.clone();
    corrupted
        .current
        .as_mut()
        .expect("Coordinator state")
        .component_scale_out_receipts[0]
        .placements[0]
        .root_receipt_content_hash[0] ^= 1;
    FleetCoordinatorRegistryStore::import(corrupted);
    let invalid = FleetCoordinatorWorkflow::registry()
        .expect_err("corrupted retired placement authority must fail closed");
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID
    );
    FleetCoordinatorRegistryStore::import(terminal.clone());

    let mut corrupted = FleetCoordinatorRegistryStore::export();
    corrupted
        .current
        .as_mut()
        .expect("Coordinator state")
        .component_scale_out_receipts[0]
        .component_count += 1;
    FleetCoordinatorRegistryStore::import(corrupted);
    let invalid = FleetCoordinatorWorkflow::registry()
        .expect_err("corrupted retired replay-only count must fail closed");
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID
    );
    FleetCoordinatorRegistryStore::import(terminal);
    assert_second_rollover_retains_ordered_history(&config, &second);
}

fn assert_second_rollover_retains_ordered_history(
    config: &ConfigModel,
    second: &FleetComponentProvisioningStatusResponse,
) {
    let registry = FleetCoordinatorWorkflow::registry().expect("second scale-out Registry");
    let third_request = FleetComponentProvisioningPrepareRequest {
        operation_id: [205; 32],
        plan: scale_out_plan(config, &registry, 3, 4),
    };
    let third = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(config, third_request, 3_000)
        .expect("retire second terminal journal and prepare third increase");
    assert_eq!(third.phase, FleetComponentProvisioningPhase::Planned);
    let rolled_twice = FleetCoordinatorRegistryStore::export();
    let current = rolled_twice.current.as_ref().expect("Coordinator state");
    assert_eq!(
        current
            .component_scale_out_receipts
            .iter()
            .map(|receipt| receipt.operation_id)
            .collect::<Vec<_>>(),
        vec![[201; 32], [203; 32]]
    );
    assert_eq!(current.component_group_deployments[0].placements.len(), 3);
    assert_eq!(
        current.component_group_deployments[0].next_placement_ordinal,
        4
    );
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::component_provisioning_status_for_test(
            config,
            FleetComponentProvisioningStatusRequest {
                operation_id: second.operation_id,
                plan_hash: second.plan_hash,
            },
        )
        .expect("second retired operation status replay"),
        *second
    );
}

fn assert_current_terminal_operation_rejects_conflicting_plan(
    config: &ConfigModel,
    request: &FleetComponentProvisioningPrepareRequest,
) {
    let before = FleetCoordinatorRegistryStore::export();
    let mut conflicting = request.clone();
    conflicting.plan.operation = FleetComponentProvisioningOperation::ScaleOut {
        deployment: "project_cells".parse().expect("deployment ID"),
        previous_placements: 1,
        requested_placements: 3,
    };
    let conflict = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(config, conflicting, 1_999)
        .expect_err("active terminal operation cannot select different plan authority");
    assert_eq!(
        conflict.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), before);
}

fn assert_next_scale_out_rejects_time_regression(
    config: &ConfigModel,
    request: &FleetComponentProvisioningPrepareRequest,
    previous: &FleetComponentProvisioningStatusResponse,
) {
    let before = FleetCoordinatorRegistryStore::export();
    let regressed_at_ns = previous
        .runtimes_activated_at_ns
        .expect("terminal activation time")
        .checked_sub(1)
        .expect("positive terminal activation time");
    let invalid = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(config, request.clone(), regressed_at_ns)
        .expect_err("next scale-out cannot predate retired terminal history");
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), before);
}

fn drive_terminal_scale_out(
    config: &ConfigModel,
    request: FleetComponentProvisioningPrepareRequest,
    started_at_ns: u64,
) -> FleetComponentProvisioningStatusResponse {
    let status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(config, request, started_at_ns)
        .expect("prepare scale-out");
    drive_prepared_scale_out(config, status, started_at_ns + 10)
}

fn drive_prepared_scale_out(
    config: &ConfigModel,
    mut status: FleetComponentProvisioningStatusResponse,
    started_at_ns: u64,
) -> FleetComponentProvisioningStatusResponse {
    let source_registry = FleetCoordinatorWorkflow::registry().expect("source Fleet Registry");
    let mut existing_service_members = BTreeMap::<Principal, u32>::new();
    for member in source_registry
        .services
        .iter()
        .flat_map(|service| &service.members)
    {
        let count = existing_service_members
            .entry(member.fleet_subnet_root)
            .or_default();
        *count = count.checked_add(1).expect("bounded service-member count");
    }
    status = drive_root_acceptance(config, status, started_at_ns);
    status = drive_root_provisioning(config, status, started_at_ns + 10);
    status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        publish_component_provisioning_services(
            &root_provision_advance_request(&status),
            started_at_ns + 100,
        )
        .expect("publish scale-out topology");
    let selected_root = FleetCoordinatorRegistryStore::export()
        .current
        .as_ref()
        .and_then(|current| current.component_scale_out.as_ref())
        .and_then(|record| record.plan.batches.first())
        .map(|batch| batch.root.fleet_subnet_root)
        .expect("selected scale-out root");
    let expected_synchronization_roots = FleetCoordinatorRegistryStore::export()
        .current
        .as_ref()
        .and_then(|current| current.component_scale_out.as_ref())
        .map(|record| record.plan.directory_confirmation_roots.clone())
        .expect("canonical scale-out synchronization roots");
    let mut synchronized_roots = Vec::new();
    let mut selected_root_published = false;
    let mut now = started_at_ns + 101;
    while status.phase != FleetComponentProvisioningPhase::DirectoriesConfirmed {
        let request = root_provision_advance_request(&status);
        let disposition = crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_directory_confirmation(&request, now)
            .expect("advance scale-out Directory barrier");
        status = match disposition {
            FleetComponentDirectoryConfirmationDisposition::Invoke(
                FleetComponentDirectoryConfirmationCallView::ScaleOutSynchronization {
                    fleet_subnet_root,
                    request: synchronization,
                },
            ) => {
                assert_eq!(
                    Some(fleet_subnet_root),
                    expected_synchronization_roots
                        .get(synchronized_roots.len())
                        .copied()
                );
                synchronized_roots.push(fleet_subnet_root);
                let affected_component_count = existing_service_members
                    .get(&fleet_subnet_root)
                    .copied()
                    .unwrap_or_default();
                let response = terminal_scale_out_synchronization_response(
                    &(fleet_subnet_root, synchronization),
                    affected_component_count,
                    now + 1,
                );
                crate::ops::fleet_coordinator::FleetCoordinatorOps::
                    record_component_scale_out_directory_synchronization(
                        &request,
                        response,
                        now + 2,
                    )
                    .expect("record scale-out root synchronization")
            }
            FleetComponentDirectoryConfirmationDisposition::Invoke(
                FleetComponentDirectoryConfirmationCallView::ScaleOutPublication {
                    fleet_subnet_root,
                    ..
                },
            ) => {
                assert_eq!(fleet_subnet_root, selected_root);
                assert!(!selected_root_published);
                selected_root_published = true;
                let response = terminal_directory_response(fleet_subnet_root, now + 1);
                crate::ops::fleet_coordinator::FleetCoordinatorOps::
                    record_component_scale_out_directory_publication(
                        &request,
                        response,
                        now + 2,
                    )
                    .expect("record selected-root Directory publication")
            }
            _ => panic!("scale-out Directory barrier returned an unexpected disposition"),
        };
        now += 10;
    }
    assert_eq!(synchronized_roots, expected_synchronization_roots);
    assert!(selected_root_published);
    drive_runtime_activation(status, started_at_ns + 200)
}

fn drive_terminal_fresh_install(config: &ConfigModel) -> FleetComponentProvisioningStatusResponse {
    drive_terminal_fresh_install_with_admission(config, 1)
}

fn drive_terminal_fresh_install_with_admission(
    config: &ConfigModel,
    maximum_root_instances: u32,
) -> FleetComponentProvisioningStatusResponse {
    let status = drive_fresh_install_to_directories_with_admission(config, maximum_root_instances);
    drive_runtime_activation(status, 300)
}

fn drive_single_root_fresh_install_to_service_topology(
    config: &ConfigModel,
) -> FleetComponentProvisioningStatusResponse {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let (_root, _version) = activate_one_root_with_config(principal(200), config, 10);
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
        .expect("prepare single-root five-Component plan");
    status = drive_root_acceptance(config, status, 110);
    status = drive_root_provisioning(config, status, 120);
    crate::ops::fleet_coordinator::FleetCoordinatorOps::publish_component_provisioning_services(
        &root_provision_advance_request(&status),
        200,
    )
    .expect("publish single-root five-Component topology")
}

fn drive_fresh_install_to_directories_with_admission(
    config: &ConfigModel,
    maximum_root_instances: u32,
) -> FleetComponentProvisioningStatusResponse {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let (_, _, _) = activate_two_roots_with_config_and_admission(
        principal(200),
        config,
        maximum_root_instances,
    );
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
    drive_directory_confirmation(status, 210)
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
                advance_component_provisioning_root_acceptance_for_test(config, &request, now)
                .expect("persist root acceptance intent"),
            false,
        );
        status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
            record_component_provisioning_root_acceptance_for_test(
                config,
                &request,
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
        let FleetComponentDirectoryConfirmationCallView::FreshPublication {
            fleet_subnet_root, ..
        } = call
        else {
            panic!("fresh Directory confirmation must publish the root batch");
        };
        let response = terminal_directory_response(fleet_subnet_root, now + 1);
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
        let advanced = crate::ops::fleet_coordinator::FleetCoordinatorOps::
            record_component_runtime_activation(&request, &response, now + 2)
            .expect("record runtime activation response");
        let FleetComponentRuntimeActivationDisposition::Current(replayed) =
            crate::ops::fleet_coordinator::FleetCoordinatorOps::
                advance_component_runtime_activation(&request, now + 3)
                .expect("replay recorded runtime activation response")
        else {
            panic!("recorded runtime activation response must replay as current")
        };
        assert_eq!(*replayed, advanced);
        status = advanced;
        now += 3;
    }
    status
}

#[test]
fn coordinator_accepts_terminal_directory_publication_after_missing_every_component() {
    let config = five_component_coordinator_config();
    let status = drive_single_root_fresh_install_to_service_topology(&config);
    assert_eq!(
        status.phase,
        FleetComponentProvisioningPhase::ServiceTopologyPublished
    );
    assert_eq!(status.root_batch_count, 1);
    assert_eq!(status.component_count, 5);

    let request = root_provision_advance_request(&status);
    let FleetComponentDirectoryConfirmationDisposition::Invoke(
        FleetComponentDirectoryConfirmationCallView::FreshPublication {
            fleet_subnet_root,
            ..
        },
    ) = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        advance_component_directory_confirmation(&request, 210)
        .expect("persist first Directory publication observation")
    else {
        panic!("first Directory publication observation must query its Root")
    };
    let durable_intent = FleetCoordinatorRegistryStore::export();
    let response = terminal_directory_response(fleet_subnet_root, 211);

    let mut invalid_responses = Vec::new();
    let mut wrong_root = response.clone();
    wrong_root.fleet_subnet_root = principal(250);
    invalid_responses.push(wrong_root);
    let mut changed_count = response.clone();
    changed_count.component_count = 6;
    invalid_responses.push(changed_count);
    let mut excessive_progress = response.clone();
    excessive_progress.published_component_count = 6;
    invalid_responses.push(excessive_progress);
    let mut wrong_receipt = response.clone();
    wrong_receipt.receipt_content_hash[0] ^= 1;
    invalid_responses.push(wrong_receipt);

    for invalid in invalid_responses {
        let conflict = crate::ops::fleet_coordinator::FleetCoordinatorOps::
            record_component_directory_confirmation(&request, invalid, 212)
            .expect_err("invalid coalesced Directory publication must fail closed");
        assert_eq!(
            conflict.public_error().code(),
            canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
        );
        assert_eq!(FleetCoordinatorRegistryStore::export(), durable_intent);
    }

    let failed = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_provisioning_root_failure(
            FleetComponentProvisioningStatusRequest {
                operation_id: request.operation_id,
                plan_hash: request.plan_hash,
            },
            canic_core::diagnostics::codes::STATE_CONFLICT
                .raw_code()
                .raw(),
            212,
        )
        .expect("retain the observed Directory confirmation failure");
    assert_eq!(
        failed
            .pending_root_failure
            .expect("pending Directory confirmation failure")
            .diagnostic_code,
        canic_core::diagnostics::codes::STATE_CONFLICT
            .raw_code()
            .raw()
    );

    let confirmed = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_directory_confirmation(&request, response, 213)
        .expect("accept fully published Root as the first Directory observation");
    assert_eq!(
        confirmed.phase,
        FleetComponentProvisioningPhase::DirectoriesConfirmed
    );
    assert_eq!(confirmed.directory_confirmed_root_count, 1);
    assert_eq!(confirmed.current_publication, None);
    assert_eq!(confirmed.publication_in_flight_root, None);
    assert_eq!(confirmed.pending_root_failure, None);

    let durable_confirmation = FleetCoordinatorRegistryStore::export();
    let FleetComponentDirectoryConfirmationDisposition::Current(replayed) =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_directory_confirmation(&request, 999)
            .expect("replay terminal Directory publication after missed observations")
    else {
        panic!("terminal Directory publication retry must return retained progress")
    };
    assert_eq!(*replayed, confirmed);
    assert_eq!(
        FleetCoordinatorRegistryStore::export(),
        durable_confirmation
    );

    let terminal = drive_runtime_activation(confirmed, 300);
    assert_eq!(
        terminal.phase,
        FleetComponentProvisioningPhase::RuntimesActivated
    );
    assert_eq!(terminal.runtime_activated_root_count, 1);
}

#[test]
fn coordinator_accepts_partial_coalesced_directory_publication_and_rejects_regression() {
    let config = five_component_coordinator_config();
    let status = drive_single_root_fresh_install_to_service_topology(&config);
    let first_request = root_provision_advance_request(&status);
    let FleetComponentDirectoryConfirmationDisposition::Invoke(
        FleetComponentDirectoryConfirmationCallView::FreshPublication {
            fleet_subnet_root,
            ..
        },
    ) = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        advance_component_directory_confirmation(&first_request, 210)
        .expect("persist first partial Directory observation")
    else {
        panic!("first partial Directory observation must query its Root")
    };

    let partial = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_directory_confirmation(
            &first_request,
            partial_directory_response(fleet_subnet_root, 3),
            212,
        )
        .expect("accept zero-to-three Directory publication progress");
    assert_eq!(
        partial.current_publication,
        Some(FleetComponentPublicationRootProgress {
            fleet_subnet_root,
            component_count: 5,
            published_component_count: 3,
        })
    );
    assert_eq!(partial.publication_in_flight_root, None);

    let durable_partial = FleetCoordinatorRegistryStore::export();
    let FleetComponentDirectoryConfirmationDisposition::Current(replayed) =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_directory_confirmation(&first_request, 999)
            .expect("replay first coalesced Directory observation")
    else {
        panic!("first coalesced Directory observation retry must return current progress")
    };
    assert_eq!(*replayed, partial);
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable_partial);

    let terminal_request = root_provision_advance_request(&partial);
    let FleetComponentDirectoryConfirmationDisposition::Invoke(
        FleetComponentDirectoryConfirmationCallView::FreshPublication { .. },
    ) = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        advance_component_directory_confirmation(&terminal_request, 220)
        .expect("persist terminal Directory observation")
    else {
        panic!("terminal Directory observation must query its Root")
    };
    let durable_terminal_intent = FleetCoordinatorRegistryStore::export();
    let regression = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_directory_confirmation(
            &terminal_request,
            partial_directory_response(fleet_subnet_root, 2),
            222,
        )
        .expect_err("Directory publication count regression must fail closed");
    assert_eq!(
        regression.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    assert_eq!(
        FleetCoordinatorRegistryStore::export(),
        durable_terminal_intent
    );

    let confirmed = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_directory_confirmation(
            &terminal_request,
            terminal_directory_response(fleet_subnet_root, 221),
            223,
        )
        .expect("accept three-to-five terminal Directory publication progress");
    assert_eq!(
        confirmed.phase,
        FleetComponentProvisioningPhase::DirectoriesConfirmed
    );

    let durable_confirmation = FleetCoordinatorRegistryStore::export();
    let FleetComponentDirectoryConfirmationDisposition::Current(replayed) =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_directory_confirmation(&terminal_request, 999)
            .expect("replay intermediate-to-terminal Directory publication")
    else {
        panic!("terminal coalesced Directory retry must return retained progress")
    };
    assert_eq!(*replayed, confirmed);
    assert_eq!(
        FleetCoordinatorRegistryStore::export(),
        durable_confirmation
    );
}

#[test]
fn scale_out_accepts_coalesced_terminal_directory_publication() {
    let config = five_component_coordinator_config();
    let status = drive_single_root_fresh_install_to_service_topology(&config);
    let status = drive_directory_confirmation(status, 210);
    let status = drive_runtime_activation(status, 300);
    assert_eq!(
        status.phase,
        FleetComponentProvisioningPhase::RuntimesActivated
    );

    let registry = FleetCoordinatorWorkflow::registry().expect("published Registry");
    let request = FleetComponentProvisioningPrepareRequest {
        operation_id: [111; 32],
        plan: scale_out_plan_on_root(&config, &registry, 1, 2, 0),
    };
    let terminal = drive_terminal_scale_out(&config, request, 1_000);
    assert_eq!(
        terminal.phase,
        FleetComponentProvisioningPhase::RuntimesActivated
    );
    assert_eq!(terminal.component_count, 5);
    assert_eq!(terminal.directory_confirmed_root_count, 1);
}

#[test]
fn coordinator_accepts_coalesced_terminal_runtime_activation_and_publishes_catalog() {
    let config = scale_out_coordinator_config();
    let status = drive_fresh_install_to_directories_with_admission(&config, 1);
    assert_eq!(
        status.phase,
        FleetComponentProvisioningPhase::DirectoriesConfirmed
    );
    assert_eq!(status.component_count, 1);

    let request = root_provision_advance_request(&status);
    let FleetComponentRuntimeActivationDisposition::Invoke(call) =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::advance_component_runtime_activation(
            &request, 300,
        )
        .expect("persist first runtime activation observation")
    else {
        panic!("first runtime activation observation must invoke its Root");
    };
    let durable_intent = FleetCoordinatorRegistryStore::export();
    let response = terminal_runtime_activation_response(call.fleet_subnet_root, 300, 301);

    let mut invalid_responses = Vec::new();
    let mut wrong_root = response.clone();
    wrong_root.fleet_subnet_root = principal(250);
    invalid_responses.push(wrong_root);
    let mut changed_count = response.clone();
    changed_count.component_count = 2;
    invalid_responses.push(changed_count);
    let mut excessive_progress = response.clone();
    excessive_progress.activated_component_count = 2;
    invalid_responses.push(excessive_progress);
    let mut early_root = response.clone();
    early_root.activated_component_count = 0;
    invalid_responses.push(early_root);
    let mut wrong_receipt = response.clone();
    wrong_receipt.receipt_content_hash[0] ^= 1;
    invalid_responses.push(wrong_receipt);
    let mut invalid_time = response.clone();
    invalid_time.activation_started_at_ns = Some(0);
    invalid_responses.push(invalid_time);

    for invalid in invalid_responses {
        let conflict = crate::ops::fleet_coordinator::FleetCoordinatorOps::
            record_component_runtime_activation(&request, &invalid, 302)
            .expect_err("invalid coalesced runtime activation must fail closed");
        assert_eq!(
            conflict.public_error().code(),
            canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
        );
        assert_eq!(FleetCoordinatorRegistryStore::export(), durable_intent);
    }

    let first_root_terminal =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::record_component_runtime_activation(
            &request, &response, 302,
        )
        .expect("accept fully terminal Root as the first activation observation");
    assert_eq!(
        first_root_terminal.phase,
        FleetComponentProvisioningPhase::ActivatingRuntimes
    );
    assert_eq!(first_root_terminal.runtime_activated_root_count, 1);
    assert_eq!(first_root_terminal.current_activation, None);
    assert_eq!(first_root_terminal.activation_in_flight_root, None);

    let durable_first_root = FleetCoordinatorRegistryStore::export();
    let FleetComponentRuntimeActivationDisposition::Current(replayed) =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::advance_component_runtime_activation(
            &request, 999,
        )
        .expect("replay terminal activation after missing every intermediate observation")
    else {
        panic!("terminal activation retry must return the retained result");
    };
    assert_eq!(*replayed, first_root_terminal);
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable_first_root);

    let terminal = drive_runtime_activation(first_root_terminal, 400);
    assert_eq!(
        terminal.phase,
        FleetComponentProvisioningPhase::RuntimesActivated
    );
    assert_eq!(terminal.runtime_activated_root_count, 2);
    let durable_terminal = FleetCoordinatorRegistryStore::export();
    let current = durable_terminal
        .current
        .as_ref()
        .expect("Coordinator state");
    assert_eq!(current.component_group_deployments.len(), 1);
    assert_eq!(current.component_group_deployments[0].placements.len(), 1);
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
        invalid.public_error().code(),
        canic_core::diagnostics::codes::REQUEST_INVALID.raw_code()
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
        wrong_status.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
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
    let referenced_root = request
        .plan
        .batches
        .iter()
        .find(|batch| !batch.placements.is_empty())
        .expect("grouped root batch")
        .root
        .fleet_subnet_root;
    let mut conflicting = request;
    conflicting.plan.directory_confirmation_roots.pop();
    let conflict = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        prepare_component_provisioning_for_test(config, conflicting, 93)
        .expect_err("one operation cannot replace its complete plan");
    assert_eq!(
        conflict.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable.clone());

    let drain =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::require_root_lifecycle_open_for_test(
            config,
            referenced_root,
        )
        .expect_err("a planned grouped Fleet fences root lifecycle");
    assert_eq!(
        drain.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
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
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID
    );
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
                &first_request,
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
                &first_request,
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
            &first_request,
            early_response,
            105,
        )
        .expect_err("root acceptance cannot predate its durable call intent");
    assert_eq!(
        invalid_time.public_error().code(),
        canic_core::diagnostics::codes::REQUEST_INVALID.raw_code()
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable_intent);

    let first_response = accepted_root_response(&first_call.request, 104);
    let first_status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_provisioning_root_acceptance_for_test(
            &config,
            &first_request,
            first_response.clone(),
            105,
        )
        .expect("record first root acceptance");
    assert_root_acceptance_status(
        &first_status,
        FleetComponentProvisioningPhase::AcceptingRoots,
        1,
    );
    assert_first_root_acceptance_replays(&config, &first_request, first_response, &first_status);

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
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID
    );
}

#[test]
fn coordinator_accepts_a_root_that_finishes_before_its_first_observation() {
    let (config, plan_hash) = prepare_two_root_acceptance_plan();
    accept_every_planned_root(&config, plan_hash);
    let status = component_provisioning_status(&config, plan_hash);
    let request = root_provision_advance_request(&status);
    let call = expect_root_provision_call(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_provisioning_root_for_test(&config, &request, 120)
            .expect("persist first root provisioning observation intent"),
        false,
    );
    let durable_intent = FleetCoordinatorRegistryStore::export();
    let response = terminal_root_provision_response(&config, &call, 119);

    let mut corrupt = response.clone();
    corrupt.receipt_content_hash[0] ^= 1;
    let conflict = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_provisioning_root_for_test(&config, &request, corrupt, 121)
        .expect_err("terminal observation still requires its exact receipt hash");
    assert_eq!(
        conflict.public_error().code(),
        canic_core::diagnostics::codes::STATE_FAILED.raw_code()
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable_intent);

    let mut post_provisioning = response.clone();
    post_provisioning.published_component_count = 1;
    let conflict = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_provisioning_root_for_test(
            &config,
            &request,
            post_provisioning,
            121,
        )
        .expect_err("Provisioned observation cannot carry publication progress");
    assert_eq!(
        conflict.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable_intent);

    let next = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_provisioning_root_for_test(
            &config,
            &request,
            response.clone(),
            121,
        )
        .expect("accept valid terminal root observation");
    assert_eq!(
        next.phase,
        FleetComponentProvisioningPhase::ProvisioningRoots
    );
    assert_eq!(next.provisioned_root_count, 1);
    assert_eq!(next.provisioning_in_flight_root, None);

    let durable = FleetCoordinatorRegistryStore::export();
    let mut corrupt = durable.clone();
    let record = corrupt
        .current
        .as_mut()
        .and_then(|current| current.component_provisioning.as_mut())
        .expect("Component provisioning record");
    let FleetComponentProvisioningStateRecord::ProvisioningRoots { provisions, .. } =
        &mut record.state
    else {
        panic!("one terminal Root must retain one provision receipt")
    };
    provisions[0].response.published_component_count = 1;
    FleetCoordinatorRegistryStore::import(corrupt);
    let invalid =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::component_provisioning_status_for_test(
            &config,
            FleetComponentProvisioningStatusRequest {
                operation_id: [101; 32],
                plan_hash,
            },
        )
        .expect_err("durable Provisioned receipt cannot carry publication progress");
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID
    );

    FleetCoordinatorRegistryStore::import(durable);
    assert_eq!(component_provisioning_status(&config, plan_hash), next);
    assert_eq!(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            record_component_provisioning_root_for_test(&config, &request, response, 999)
            .expect("replay terminal observation after Coordinator commit"),
        next
    );
}

#[test]
fn coordinator_normalizes_a_root_that_advances_before_acceptance_observation() {
    for terminal in [false, true] {
        let (config, plan_hash) = prepare_two_root_acceptance_plan();
        let request = root_acceptance_advance_request(plan_hash, 0);
        let call = expect_root_acceptance_call(
            crate::ops::fleet_coordinator::FleetCoordinatorOps::
                advance_component_provisioning_root_acceptance_for_test(&config, &request, 103)
                .expect("persist first root acceptance intent"),
            false,
        );
        let response = if terminal {
            terminal_root_acceptance_observation(&config, &call.request, 104, 104)
        } else {
            let mut progressed = accepted_root_response(&call.request, 104);
            progressed.reserved_component_count = 1;
            progressed
        };
        if !terminal {
            let durable_intent = FleetCoordinatorRegistryStore::export();
            let mut invalid = response.clone();
            invalid.published_component_count = 1;
            let conflict = crate::ops::fleet_coordinator::FleetCoordinatorOps::
                record_component_provisioning_root_acceptance_for_test(
                    &config, &request, invalid, 105,
                )
                .expect_err("pre-provisioning observation cannot carry publication progress");
            assert_eq!(
                conflict.public_error().code(),
                canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
            );
            assert_eq!(FleetCoordinatorRegistryStore::export(), durable_intent);
        }
        let status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
            record_component_provisioning_root_acceptance_for_test(
                &config,
                &request,
                response.clone(),
                105,
            )
            .expect("normalize progressed root acceptance observation");
        assert_root_acceptance_status(&status, FleetComponentProvisioningPhase::AcceptingRoots, 1);

        let durable = FleetCoordinatorRegistryStore::export();
        let record = durable
            .current
            .as_ref()
            .and_then(|current| current.component_provisioning.as_ref())
            .expect("Component provisioning record");
        let FleetComponentProvisioningStateRecord::AcceptingRoots { acceptances, .. } =
            &record.state
        else {
            panic!("first root acceptance must remain in progress")
        };
        assert_eq!(acceptances.len(), 1);
        assert_eq!(
            acceptances[0].response,
            accepted_root_response(&call.request, 104)
        );
        assert_first_root_acceptance_replays(&config, &request, response, &status);
    }
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
                    &request,
                    110 + u64::from(index) * 3,
                )
                .expect("persist root acceptance intent"),
            false,
        );
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            record_component_provisioning_root_acceptance_for_test(
                config,
                &request,
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
        conflict.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
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
        early.public_error().code(),
        canic_core::diagnostics::codes::REQUEST_INVALID.raw_code()
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
        conflict.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
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
        expected_current_synchronization: status.current_synchronization,
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
    request: &FleetComponentProvisioningAdvanceRequest,
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
        expected_current_synchronization: None,
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

fn terminal_root_acceptance_observation(
    config: &ConfigModel,
    request: &RootComponentProvisioningAcceptanceRequest,
    accepted_at_ns: u64,
    provisioned_at_ns: u64,
) -> RootComponentProvisioningStatusResponse {
    let durable = FleetCoordinatorRegistryStore::export();
    let record = durable
        .current
        .as_ref()
        .and_then(|current| current.component_provisioning.as_ref())
        .expect("Component provisioning record");
    provisioned_root_response(
        config,
        record,
        &request.batch,
        accepted_at_ns,
        provisioned_at_ns,
    )
}

fn terminal_root_provision_response(
    config: &ConfigModel,
    call: &FleetComponentProvisioningRootProvisionCallView,
    provisioned_at_ns: u64,
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
    let accepted_at_ns = provisioning_acceptances(&record.state)[root_index]
        .response
        .accepted_at_ns;
    provisioned_root_response(config, record, batch, accepted_at_ns, provisioned_at_ns)
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

fn provisioned_group_member(
    batch: &FleetSubnetRootProvisioningBatch,
    entry: &ComponentGroupPlanEntry,
    role: CanisterRole,
    identity_byte: u8,
) -> RootProvisionedGroupMember {
    RootProvisionedGroupMember {
        member_operation_id: [identity_byte.wrapping_add(64); 32],
        member_path: entry.member_path.clone(),
        component_spec: entry.component_spec.clone(),
        purpose: entry.purpose.clone(),
        limits: entry.limits.clone(),
        binding: ComponentBinding {
            authority: batch.root.authority.clone(),
            component: ComponentInstanceId::from_generated_bytes([identity_byte; 32]),
            component_spec: entry.component_spec.clone(),
            spec_hash: entry.spec_hash,
            role,
            placement_subnet: batch.root.placement_subnet,
            fleet_subnet_root: batch.root.fleet_subnet_root,
            canister_id: principal(identity_byte),
        },
        component_registry_revision: u64::from(identity_byte),
        component_registry_content_hash: [identity_byte; 32],
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
    let mut next_identity_byte = identity_byte;
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
                        let member_identity_byte = next_identity_byte;
                        next_identity_byte = next_identity_byte
                            .checked_add(1)
                            .expect("test member identity byte");
                        provisioned_group_member(
                            batch,
                            entry,
                            spec.component_role.clone(),
                            member_identity_byte,
                        )
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

fn expect_scale_out_synchronization_call(
    disposition: FleetComponentDirectoryConfirmationDisposition,
) -> (
    Principal,
    canic_core::dto::component_provisioning::RootComponentDirectorySynchronizationRequest,
) {
    match disposition {
        FleetComponentDirectoryConfirmationDisposition::Invoke(
            FleetComponentDirectoryConfirmationCallView::ScaleOutSynchronization {
                fleet_subnet_root,
                request,
            },
        )
        | FleetComponentDirectoryConfirmationDisposition::Reconcile(
            FleetComponentDirectoryConfirmationCallView::ScaleOutSynchronization {
                fleet_subnet_root,
                request,
            },
        ) => (fleet_subnet_root, request),
        _ => panic!("scale-out Directory barrier must synchronize the current root"),
    }
}

fn expect_scale_out_publication_call(
    disposition: FleetComponentDirectoryConfirmationDisposition,
) -> (Principal, RootComponentPublicationRequest) {
    match disposition {
        FleetComponentDirectoryConfirmationDisposition::Invoke(
            FleetComponentDirectoryConfirmationCallView::ScaleOutPublication {
                fleet_subnet_root,
                request,
            },
        )
        | FleetComponentDirectoryConfirmationDisposition::Reconcile(
            FleetComponentDirectoryConfirmationCallView::ScaleOutPublication {
                fleet_subnet_root,
                request,
            },
        ) => (fleet_subnet_root, request),
        _ => panic!("scale-out Directory barrier must publish the selected root"),
    }
}

fn confirm_selected_scale_out_root(
    mut status: FleetComponentProvisioningStatusResponse,
    selected_root: Principal,
    started_at_ns: u64,
) -> FleetComponentProvisioningStatusResponse {
    let confirmed_before = status.directory_confirmed_root_count;
    let synchronization_request = root_provision_advance_request(&status);
    let synchronization_call = expect_scale_out_synchronization_call(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_directory_confirmation(
                &synchronization_request,
                started_at_ns,
            )
            .expect("persist selected-root synchronization"),
    );
    assert_eq!(synchronization_call.0, selected_root);
    let synchronization_response =
        terminal_scale_out_synchronization_response(&synchronization_call, 0, started_at_ns + 1);
    let durable_synchronization_intent = FleetCoordinatorRegistryStore::export();
    FleetCoordinatorRegistryStore::import(durable_synchronization_intent.clone());
    let synchronization_replay = expect_scale_out_synchronization_call(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_directory_confirmation(&synchronization_request, 9_999)
            .expect("reconcile selected-root synchronization after restart"),
    );
    assert_eq!(synchronization_replay, synchronization_call);
    assert_eq!(
        FleetCoordinatorRegistryStore::export(),
        durable_synchronization_intent
    );
    status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_scale_out_directory_synchronization(
            &synchronization_request,
            synchronization_response,
            started_at_ns + 2,
        )
        .expect("record selected-root synchronization");
    assert_eq!(status.directory_confirmed_root_count, confirmed_before);
    assert!(status.current_synchronization.is_some());
    assert!(status.current_publication.is_none());
    let durable_synchronization = FleetCoordinatorRegistryStore::export();
    FleetCoordinatorRegistryStore::import(durable_synchronization.clone());
    let FleetComponentDirectoryConfirmationDisposition::Current(replayed) =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_directory_confirmation(&synchronization_request, 9_999)
            .expect("replay committed selected-root synchronization after restart")
    else {
        panic!("an exact pre-synchronization command must replay committed progress")
    };
    assert_eq!(*replayed, status);
    assert_eq!(
        FleetCoordinatorRegistryStore::export(),
        durable_synchronization
    );
    assert_conflicting_synchronization_cursor_rejects(&status);

    let publication_request = root_provision_advance_request(&status);
    let publication_call = expect_scale_out_publication_call(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_directory_confirmation(&publication_request, started_at_ns + 10)
            .expect("persist selected-root publication"),
    );
    assert_eq!(publication_call.0, selected_root);
    let publication_response = terminal_directory_response(selected_root, started_at_ns + 11);
    let durable_publication_intent = FleetCoordinatorRegistryStore::export();
    FleetCoordinatorRegistryStore::import(durable_publication_intent.clone());
    let publication_replay = expect_scale_out_publication_call(
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_directory_confirmation(&publication_request, 9_999)
            .expect("reconcile selected-root publication after restart"),
    );
    assert_eq!(publication_replay, publication_call);
    assert_eq!(
        FleetCoordinatorRegistryStore::export(),
        durable_publication_intent
    );
    let status = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_scale_out_directory_publication(
            &publication_request,
            publication_response,
            started_at_ns + 12,
        )
        .expect("record selected-root publication");
    let durable_publication = FleetCoordinatorRegistryStore::export();
    FleetCoordinatorRegistryStore::import(durable_publication.clone());
    let FleetComponentDirectoryConfirmationDisposition::Current(replayed) =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::
            advance_component_directory_confirmation(&publication_request, 9_999)
            .expect("replay committed selected-root publication after restart")
    else {
        panic!("an exact pre-publication command must replay committed progress")
    };
    assert_eq!(*replayed, status);
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable_publication);
    status
}

fn assert_conflicting_synchronization_cursor_rejects(
    status: &FleetComponentProvisioningStatusResponse,
) {
    let mut request = root_provision_advance_request(status);
    let mut cursor = request
        .expected_current_synchronization
        .expect("scale-out synchronization cursor");
    cursor.fleet_subnet_root = principal(250);
    request.expected_current_synchronization = Some(cursor);
    let durable = FleetCoordinatorRegistryStore::export();
    let Err(invalid) = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        advance_component_directory_confirmation(&request, 9_999)
    else {
        panic!("a substituted synchronization root must reject")
    };
    assert_eq!(
        invalid.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable);
}

fn activate_selected_scale_out_root(
    mut status: FleetComponentProvisioningStatusResponse,
    selected_root: Principal,
    affected_only_root: Principal,
    started_at_ns: u64,
) -> FleetComponentProvisioningStatusResponse {
    let request = root_provision_advance_request(&status);
    let FleetComponentRuntimeActivationDisposition::Invoke(call) =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::advance_component_runtime_activation(
            &request,
            started_at_ns,
        )
        .expect("persist selected-root runtime activation intent")
    else {
        panic!("scale-out runtime activation must invoke its selected root");
    };
    assert_eq!(call.fleet_subnet_root, selected_root);
    assert_ne!(call.fleet_subnet_root, affected_only_root);
    let response =
        next_runtime_activation_response(selected_root, started_at_ns, started_at_ns + 1);
    let durable_intent = FleetCoordinatorRegistryStore::export();
    FleetCoordinatorRegistryStore::import(durable_intent.clone());
    let FleetComponentRuntimeActivationDisposition::Reconcile(replayed) =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::advance_component_runtime_activation(
            &request, 9_999,
        )
        .expect("reconcile selected-root runtime activation intent")
    else {
        panic!("lost scale-out activation response must reconcile");
    };
    assert_eq!(replayed.fleet_subnet_root, call.fleet_subnet_root);
    assert_eq!(replayed.request, call.request);
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable_intent);

    status =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::record_component_runtime_activation(
            &request,
            &response,
            started_at_ns + 2,
        )
        .expect("record selected-root runtime activation progress");
    status = drive_runtime_activation(status, started_at_ns + 10);

    let terminal = FleetCoordinatorRegistryStore::export();
    let replay =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::advance_component_runtime_activation(
            &root_provision_advance_request(&status),
            10_000,
        )
        .expect("replay terminal scale-out activation");
    assert!(matches!(
        replay,
        FleetComponentRuntimeActivationDisposition::Current(_)
    ));
    assert_eq!(FleetCoordinatorRegistryStore::export(), terminal);
    status
}

fn assert_terminal_scale_out_placement(
    status: &FleetComponentProvisioningStatusResponse,
    selected_root: Principal,
) {
    let terminal = FleetCoordinatorRegistryStore::export();
    let current = terminal.current.as_ref().expect("Coordinator state");
    let scale_out = current
        .component_scale_out
        .as_ref()
        .expect("terminal scale-out operation");
    let FleetComponentProvisioningStateRecord::RuntimesActivated { activations, .. } =
        &scale_out.state
    else {
        panic!("scale-out runtime activation must be terminal")
    };
    assert_eq!(activations.len(), 1);
    assert_eq!(activations[0].progress.fleet_subnet_root, selected_root);
    let committed = current
        .component_group_deployments
        .iter()
        .flat_map(|deployment| &deployment.placements)
        .find(|placement| placement.operation_id == status.operation_id)
        .expect("scale-out placement committed to deployment ledger");
    assert_eq!(committed.fleet_subnet_root, selected_root);
    assert_eq!(
        committed.root_receipt_content_hash,
        activations[0].receipt_content_hash
    );
}

fn terminal_scale_out_synchronization_response(
    call: &(
        Principal,
        canic_core::dto::component_provisioning::RootComponentDirectorySynchronizationRequest,
    ),
    affected_component_count: u32,
    synchronized_at_ns: u64,
) -> RootComponentDirectorySynchronizationResponse {
    let current = FleetCoordinatorRegistryStore::export()
        .current
        .expect("Coordinator state");
    let directory = FleetRegistryOps::directory_for_root(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &current.registry,
        call.0,
    )
    .expect("root Fleet Directory");
    assert_eq!(
        directory.provenance.registry,
        call.1.published_fleet_registry
    );
    let mut response = RootComponentDirectorySynchronizationResponse {
        operation_id: call.1.operation_id,
        plan_hash: call.1.plan_hash,
        source_fleet_registry: call.1.source_fleet_registry.clone(),
        published_fleet_registry: call.1.published_fleet_registry.clone(),
        fleet_subnet_root: call.0,
        affected_component_count,
        synchronized_component_count: affected_component_count,
        fleet_directory_content_hash:
            RootComponentProvisioningReceiptOps::fleet_directory_content_hash(&directory)
                .expect("Fleet Directory hash"),
        complete: true,
        synchronized_at_ns: Some(synchronized_at_ns),
        receipt_content_hash: [0; 32],
    };
    response.receipt_content_hash =
        RootComponentProvisioningReceiptOps::directory_synchronization_content_hash(&response)
            .expect("Directory synchronization receipt hash");
    response
}

fn terminal_directory_response(
    fleet_subnet_root: Principal,
    published_at_ns: u64,
) -> RootComponentProvisioningStatusResponse {
    directory_response(fleet_subnet_root, None, Some(published_at_ns))
}

fn partial_directory_response(
    fleet_subnet_root: Principal,
    published_component_count: u32,
) -> RootComponentProvisioningStatusResponse {
    directory_response(fleet_subnet_root, Some(published_component_count), None)
}

fn directory_response(
    fleet_subnet_root: Principal,
    published_component_count: Option<u32>,
    published_at_ns: Option<u64>,
) -> RootComponentProvisioningStatusResponse {
    let durable = FleetCoordinatorRegistryStore::export();
    let current = durable.current.as_ref().expect("Coordinator state");
    let record = current
        .component_scale_out
        .as_ref()
        .filter(|record| {
            matches!(
                record.state,
                FleetComponentProvisioningStateRecord::ConfirmingDirectories { .. }
            )
        })
        .or(current.component_provisioning.as_ref())
        .expect("Directory provisioning record");
    let context = directory_publication_context(record);
    let batch = &record.plan.batches[context.root_index];
    assert_eq!(batch.root.fleet_subnet_root, fleet_subnet_root);
    let result = context.previous.result.clone().expect("provisioned result");
    let published_component_count =
        published_component_count.unwrap_or(context.previous.component_count);
    let component_directories = result
        .placements
        .iter()
        .flat_map(|placement| &placement.members)
        .take(usize::try_from(published_component_count).expect("published Component count"))
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
    assert_eq!(
        fleet_directory.provenance.registry,
        context.published_registry
    );
    let publication = RootComponentPublicationEvidence {
        fleet_registry: context.published_registry,
        fleet_directory_content_hash:
            RootComponentProvisioningReceiptOps::fleet_directory_content_hash(&fleet_directory)
                .expect("Fleet Directory hash"),
        component_directories,
        component_group_directories,
    };
    let receipt_content_hash = published_at_ns.map(|published_at_ns| {
        RootComponentProvisioningReceiptOps::published_content_hash(
            RootComponentProvisioningPublishedReceiptAuthority {
                operation_id: record.operation_id,
                plan_hash: record.plan_hash,
                configuration_digest: record.plan.configuration_digest,
                root: &batch.root,
                result: &result,
                publication: &publication,
                accepted_at_ns: context.previous.accepted_at_ns,
                provisioned_at_ns: context
                    .previous
                    .provisioned_at_ns
                    .expect("provisioned time"),
                published_at_ns,
            },
        )
        .expect("published receipt hash")
    });
    let mut response = context.previous;
    response.published_component_count = published_component_count;
    response.publication = Some(publication);
    if let Some(published_at_ns) = published_at_ns {
        response.phase = RootComponentProvisioningPhase::Published;
        response.published_at_ns = Some(published_at_ns);
        response.receipt_content_hash = receipt_content_hash.expect("terminal receipt hash");
    }
    response
}

struct DirectoryPublicationTestContext {
    root_index: usize,
    previous: RootComponentProvisioningStatusResponse,
    published_registry: canic_core::dto::fleet_registry::FleetRegistryVersion,
}

fn directory_publication_context(
    record: &crate::storage::stable::fleet_coordinator::FleetComponentProvisioningRecord,
) -> DirectoryPublicationTestContext {
    let FleetComponentProvisioningStateRecord::ConfirmingDirectories {
        provisions,
        published_fleet_registry,
        confirmations,
        current,
        in_flight: Some(intent),
        ..
    } = &record.state
    else {
        panic!("Directory response requires an in-flight confirmation");
    };
    let (root_index, previous) = match intent.as_ref() {
        FleetComponentDirectoryConfirmationIntentRecord::FreshPublication {
            root_index, ..
        } => {
            let root_index = usize::try_from(*root_index).expect("root index");
            assert_eq!(confirmations.len(), root_index);
            let previous = current.as_ref().map_or_else(
                || provisions[root_index].response.clone(),
                |record| match record.as_ref() {
                    FleetComponentDirectoryConfirmationRecord::FreshPublication {
                        response,
                        ..
                    } => response.as_ref().clone(),
                    FleetComponentDirectoryConfirmationRecord::ScaleOut { .. } => {
                        panic!("fresh Directory response retained scale-out evidence");
                    }
                },
            );
            (root_index, previous)
        }
        FleetComponentDirectoryConfirmationIntentRecord::ScaleOutPublication {
            fleet_subnet_root,
            ..
        } => {
            let root_index = record
                .plan
                .batches
                .iter()
                .position(|batch| batch.root.fleet_subnet_root == *fleet_subnet_root)
                .expect("selected scale-out root batch");
            let previous = current.as_ref().map_or_else(
                || provisions[root_index].response.clone(),
                |record| match record.as_ref() {
                    FleetComponentDirectoryConfirmationRecord::ScaleOut {
                        publication: Some(response),
                        ..
                    } => response.as_ref().clone(),
                    FleetComponentDirectoryConfirmationRecord::ScaleOut {
                        publication: None,
                        ..
                    } => provisions[root_index].response.clone(),
                    FleetComponentDirectoryConfirmationRecord::FreshPublication { .. } => {
                        panic!("scale-out Directory retained fresh evidence");
                    }
                },
            );
            (root_index, previous)
        }
        FleetComponentDirectoryConfirmationIntentRecord::ScaleOutSynchronization { .. } => {
            panic!("Directory publication response requires publication intent");
        }
    };
    DirectoryPublicationTestContext {
        root_index,
        previous,
        published_registry: published_fleet_registry.clone(),
    }
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
    runtime_activation_response(
        fleet_subnet_root,
        activation_started_at_ns,
        observed_at_ns,
        false,
    )
}

fn terminal_runtime_activation_response(
    fleet_subnet_root: Principal,
    activation_started_at_ns: u64,
    observed_at_ns: u64,
) -> RootComponentProvisioningStatusResponse {
    runtime_activation_response(
        fleet_subnet_root,
        activation_started_at_ns,
        observed_at_ns,
        true,
    )
}

fn runtime_activation_response(
    fleet_subnet_root: Principal,
    activation_started_at_ns: u64,
    observed_at_ns: u64,
    terminal: bool,
) -> RootComponentProvisioningStatusResponse {
    let durable = FleetCoordinatorRegistryStore::export();
    let current = durable.current.as_ref().expect("Coordinator state");
    let record = current
        .component_scale_out
        .as_ref()
        .filter(|record| {
            matches!(
                record.state,
                FleetComponentProvisioningStateRecord::ActivatingRuntimes { .. }
            )
        })
        .or(current.component_provisioning.as_ref())
        .expect("runtime-activation provisioning record");
    let context = runtime_activation_test_context(record, activation_started_at_ns);
    let RuntimeActivationTestContext {
        root_index,
        publication,
        activated_component_count,
        component_count,
        activation_started_at_ns: durable_started_at_ns,
    } = context;
    assert_eq!(publication.fleet_subnet_root, fleet_subnet_root);
    if !terminal && activated_component_count < component_count {
        let mut response = publication;
        response.activated_component_count = activated_component_count + 1;
        response.activation_started_at_ns = Some(durable_started_at_ns);
        return response;
    }

    let batch = &record.plan.batches[root_index];
    let root_activated_at_ns = match &record.plan.operation {
        FleetComponentProvisioningOperation::FreshInstall => observed_at_ns,
        FleetComponentProvisioningOperation::ScaleOut { .. } => publication
            .accepted_at_ns
            .checked_sub(1)
            .expect("active root predates scale-out acceptance"),
    };
    let activation = RootComponentActivationEvidence {
        fleet_activation_operation_id: [fleet_subnet_root.as_slice()[0]; 32],
        initial_inventory_hash: [fleet_subnet_root.as_slice()[1]; 32],
        component_count,
        root_activated_at_ns,
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

struct RuntimeActivationTestContext {
    root_index: usize,
    publication: RootComponentProvisioningStatusResponse,
    activated_component_count: u32,
    component_count: u32,
    activation_started_at_ns: u64,
}

fn runtime_activation_test_context(
    record: &crate::storage::stable::fleet_coordinator::FleetComponentProvisioningRecord,
    default_started_at_ns: u64,
) -> RuntimeActivationTestContext {
    let FleetComponentProvisioningStateRecord::ActivatingRuntimes {
        confirmations,
        activations,
        current,
        in_flight: Some(intent),
        ..
    } = &record.state
    else {
        panic!("runtime response requires an in-flight activation")
    };
    let root_index = activations.len();
    assert_eq!(
        usize::try_from(intent.root_index).expect("root index"),
        root_index
    );
    let activation_root = record.plan.batches[root_index].root.fleet_subnet_root;
    let publication = confirmations
        .iter()
        .filter_map(confirmation_publication_response_for_test)
        .find(|response| response.fleet_subnet_root == activation_root)
        .cloned()
        .expect("selected root publication response");
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
    let activation_started_at_ns = current
        .as_ref()
        .and_then(|record| record.activation_started_at_ns)
        .unwrap_or(default_started_at_ns);
    RuntimeActivationTestContext {
        root_index,
        publication,
        activated_component_count,
        component_count,
        activation_started_at_ns,
    }
}

fn confirmation_publication_response_for_test(
    confirmation: &FleetComponentDirectoryConfirmationRecord,
) -> Option<&RootComponentProvisioningStatusResponse> {
    match confirmation {
        FleetComponentDirectoryConfirmationRecord::FreshPublication { response, .. } => {
            Some(response.as_ref())
        }
        FleetComponentDirectoryConfirmationRecord::ScaleOut { publication, .. } => {
            publication.as_deref()
        }
    }
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
            advance_component_provisioning_root_acceptance_for_test(config, &request, 106)
            .expect("journal second root intent"),
        false,
    );
    let durable_intent = FleetCoordinatorRegistryStore::export();
    let mut substituted = accepted_root_response(&call.request, 107);
    substituted.fleet_subnet_root = principal(108);
    let conflict = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_provisioning_root_acceptance_for_test(
            config,
            &request,
            substituted,
            108,
        )
        .expect_err("substituted root response must reject");
    assert_eq!(
        conflict.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), durable_intent);

    let complete = crate::ops::fleet_coordinator::FleetCoordinatorOps::
        record_component_provisioning_root_acceptance_for_test(
            config,
            &request,
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
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID
    );
    FleetCoordinatorRegistryStore::import(exact);
}

fn activate_one_root_with_config(
    coordinator: Principal,
    config: &ConfigModel,
    maximum_root_instances: u32,
) -> (FleetSubnetRootEntry, FleetRegistryVersion) {
    let args = init_args_with_config(coordinator, config);
    let topology = args
        .component_deployment_configuration
        .component_topology
        .clone();
    FleetCoordinatorWorkflow::initialize(args, principal(60), true, coordinator)
        .expect("commit genesis");
    let root = joining_entry(&topology, 61, 62, maximum_root_instances);
    let joined = FleetCoordinatorWorkflow::join_root(FleetSubnetRootJoinRequest {
        expected_registry: FleetCoordinatorWorkflow::version().expect("genesis version"),
        entry: root.clone(),
    })
    .expect("join root");
    FleetCoordinatorWorkflow::acknowledge_root_snapshot(
        root.fleet_subnet_root,
        canic_core::dto::fleet_registry::FleetSubnetRootSnapshotAcknowledgementRequest {
            version: joined.version.clone(),
        },
    )
    .expect("acknowledge joining snapshot");
    let active = FleetCoordinatorWorkflow::activate_registry(FleetRegistryActivationRequest {
        expected_registry: joined.version,
    })
    .expect("activate root");
    (root, active.version)
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
    activate_two_roots_with_config_and_admission(coordinator, config, 1)
}

fn activate_two_roots_with_config_and_admission(
    coordinator: Principal,
    config: &ConfigModel,
    maximum_root_instances: u32,
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
    let first = joining_entry(&topology, 61, 62, maximum_root_instances);
    let second = joining_entry(&topology, 63, 64, maximum_root_instances);
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
                funding: root.funding.clone(),
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
    scale_out_plan(config, registry, 1, 2)
}

fn scale_out_plan(
    config: &ConfigModel,
    registry: &FleetRegistry,
    previous_placements: u32,
    requested_placements: u32,
) -> FleetComponentProvisioningPlan {
    scale_out_plan_on_root(
        config,
        registry,
        previous_placements,
        requested_placements,
        1,
    )
}

fn scale_out_plan_on_root(
    config: &ConfigModel,
    registry: &FleetRegistry,
    previous_placements: u32,
    requested_placements: u32,
    root_index: usize,
) -> FleetComponentProvisioningPlan {
    let (deployment, component_group, entries) = project_cell_plan_entries(config);
    let root = &registry.fleet_subnet_roots[root_index];
    let mut plan = component_plan(
        config,
        registry,
        FleetComponentProvisioningOperation::ScaleOut {
            deployment: deployment.clone(),
            previous_placements,
            requested_placements,
        },
        vec![FleetSubnetRootProvisioningBatch {
            root: fleet_subnet_root_binding(registry, root),
            active_release_set: root.active_release_set,
            placements: (previous_placements..requested_placements)
                .map(|ordinal| ComponentGroupPlacementPlan {
                    group_placement: ComponentGroupPlacementId {
                        deployment: deployment.clone(),
                        ordinal,
                    },
                    component_group: component_group.clone(),
                    entries: entries.clone(),
                })
                .collect(),
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
        funding: root.funding.clone(),
        limits: root.limits.clone(),
    }
}

fn assert_snapshot_acknowledgements(
    registry: &FleetRegistry,
    first_entry: &FleetSubnetRootEntry,
    second_entry: &FleetSubnetRootEntry,
    version: &FleetRegistryVersion,
) -> FleetRegistryVersion {
    FleetCoordinatorWorkflow::authorize_registry_caller(first_entry.fleet_subnet_root, false)
        .expect("registered Root authorization");
    let unauthorized_registry =
        FleetCoordinatorWorkflow::authorize_registry_caller(principal(99), false)
            .expect_err("unregistered caller must fail before Registry dispatch");
    assert_eq!(
        unauthorized_registry.public_error().code(),
        canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
    );
    let snapshot =
        FleetCoordinatorWorkflow::registry_for_caller(first_entry.fleet_subnet_root, false)
            .expect("registered root Registry");
    assert_eq!(&snapshot, registry);
    assert_eq!(
        &FleetCoordinatorWorkflow::version().expect("Registry version"),
        version
    );
    let unauthorized_snapshot = FleetCoordinatorWorkflow::registry_for_caller(principal(99), false)
        .expect_err("unregistered caller cannot fetch Registry");
    assert_eq!(
        unauthorized_snapshot.public_error().code(),
        canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
    );

    let request = canic_core::dto::fleet_registry::FleetSubnetRootSnapshotAcknowledgementRequest {
        version: version.clone(),
    };
    FleetCoordinatorWorkflow::authorize_root_snapshot_caller(first_entry.fleet_subnet_root)
        .expect("joining Root authorization");
    let unauthorized_acknowledgement =
        FleetCoordinatorWorkflow::authorize_root_snapshot_caller(principal(99))
            .expect_err("unregistered caller must fail before acknowledgement dispatch");
    assert_eq!(
        unauthorized_acknowledgement.public_error().code(),
        canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
    );
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
        incomplete.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
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
    let reservation =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::prepare_root_draining_reservation(
            root_draining_reservation_request(first_entry, active_version, [21; 32]),
            20,
        )
        .expect("prepare first root draining reservation");
    let request = FleetSubnetRootDrainingPublicationRequest {
        expected_registry: active_version.clone(),
        root_draining: FleetSubnetRootDrainingResponse {
            operation_id: [21; 32],
            fleet_subnet_root: first_entry.fleet_subnet_root,
            placement_subnet: first_entry.placement_subnet,
            active_registry: active_version.clone(),
            reservation_hash: reservation.reservation_hash,
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
        invalid.public_error().code(),
        canic_core::diagnostics::codes::REQUEST_INVALID.raw_code()
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
        conflict.public_error().code(),
        canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
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
    assert_eq!(
        invalid.code(),
        canic_core::diagnostics::codes::STATE_INVALID.raw_code()
    );
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
            wasm_store_template_count: 3,
            wasm_store_release_count: 3,
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
        unauthorized.public_error().code(),
        canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
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
        FleetCoordinatorWorkflow::registry_for_caller(first_entry.fleet_subnet_root, false)
            .expect_err("Removed root cannot fetch a later Registry");
    assert_eq!(
        removed_snapshot.public_error().code(),
        canic_core::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
    );
    let surviving_snapshot =
        FleetCoordinatorWorkflow::registry_for_caller(second_entry.fleet_subnet_root, false)
            .expect("surviving root can fetch Registry containing Removed peer");
    assert_eq!(surviving_snapshot, registry);
    assert_eq!(
        FleetCoordinatorWorkflow::version().expect("Removed Registry version"),
        removed.version
    );
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
    let reservation =
        crate::ops::fleet_coordinator::FleetCoordinatorOps::prepare_root_draining_reservation(
            root_draining_reservation_request(second_entry, removed_version, [31; 32]),
            31,
        )
        .expect("prepare later root draining reservation");
    let request = FleetSubnetRootDrainingPublicationRequest {
        expected_registry: removed_version.clone(),
        root_draining: FleetSubnetRootDrainingResponse {
            operation_id: [31; 32],
            fleet_subnet_root: second_entry.fleet_subnet_root,
            placement_subnet: second_entry.placement_subnet,
            active_registry: removed_version.clone(),
            reservation_hash: reservation.reservation_hash,
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
        funding: crate::test_support::fleet_subnet_root_funding_authority(),
        limits: FleetSubnetRootLimits {
            maximum_component_instances: maximum_root_instances.max(3),
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

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one end-to-end staged rotation and interruption replay proof"
)]
fn coordinator_policy_rotation_converges_once_and_preserves_application_registry_state() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let coordinator = principal(68);
    let (first, second, predecessor_registry) = activate_two_roots(coordinator);
    let application_before = FleetCoordinatorWorkflow::registry().expect("predecessor Registry");
    let window_ns = first
        .funding
        .root_funding
        .budget
        .window_secs
        .checked_mul(1_000_000_000)
        .expect("window nanoseconds");
    for operation_sequence in 1..=first.funding.root_funding.maximum_automatic_grants {
        let now_ns = u64::from(operation_sequence)
            .checked_mul(window_ns)
            .expect("grant observation time");
        let request = root_funding_request(
            coordinator,
            &first,
            predecessor_registry.clone(),
            u64::from(operation_sequence),
            0,
        );
        let call = match FleetCoordinatorOps::prepare_root_funding(
            first.fleet_subnet_root,
            request,
            2_000_000_000_000_000,
            now_ns,
        )
        .expect("prepare reachable predecessor grant")
        {
            FleetRootFundingDisposition::Invoke(call) => call,
            FleetRootFundingDisposition::Current(_) | FleetRootFundingDisposition::Reconcile(_) => {
                panic!("fresh predecessor grant must invoke")
            }
        };
        let receipt = FleetRootFundingAcceptanceReceipt {
            request: call.request.clone(),
            fleet_subnet_root: first.fleet_subnet_root,
            coordinator,
            accepted_at_ns: now_ns + 1,
        };
        assert!(matches!(
            FleetCoordinatorOps::record_root_funding_acceptance(
                first.fleet_subnet_root,
                &call.request,
                receipt,
                now_ns + 2,
            )
            .expect("commit reachable predecessor grant"),
            FleetRootFundingResponse::Granted(_)
        ));
    }
    let exhausted = FleetCoordinatorFundingStore::export()
        .current
        .expect("exhausted Coordinator funding state");
    let exhausted_root = exhausted.roots.first().expect("exhausted Root ledger");
    let mut plan = funding_rotation_plan(predecessor_registry.clone(), [&first, &second]);
    let exhausted_usage = FleetFundingPolicyUsage {
        historical_automatic_grants: exhausted.historical_automatic_grants,
        historical_automatic_cycles: exhausted.historical_automatic_cycles.clone(),
        generation_automatic_grants: exhausted.automatic_grants,
        generation_automatic_cycles: exhausted.automatic_cycles.clone(),
    };
    plan.header.predecessor_usage = exhausted_usage;
    plan.roots
        .iter_mut()
        .find(|root| root.fleet_subnet_root == first.fleet_subnet_root)
        .expect("first Root plan")
        .predecessor_usage = FleetFundingPolicyUsage {
        historical_automatic_grants: exhausted_root.historical_automatic_grants,
        historical_automatic_cycles: exhausted_root.historical_automatic_cycles.clone(),
        generation_automatic_grants: exhausted_root.automatic_grants,
        generation_automatic_cycles: exhausted_root.automatic_cycles.clone(),
    };
    plan.header.roots_digest = fleet_funding_policy_rotation_roots_digest(&plan.roots);
    let plan_digest = fleet_funding_policy_rotation_plan_digest(&plan);
    let operation_id = fleet_funding_policy_rotation_operation_id(coordinator, plan_digest);
    let begin = FleetFundingPolicyRotationBeginRequest {
        operation_id,
        plan_digest,
        header: plan.header.clone(),
    };

    let before = FleetCoordinatorFundingStore::export();
    let mut stale_begin = begin.clone();
    stale_begin.header.predecessor_generation += 1;
    FleetCoordinatorOps::begin_funding_policy_rotation(stale_begin, 10)
        .expect_err("stale predecessor generation must reject");
    assert_eq!(FleetCoordinatorFundingStore::export(), before);

    FleetCoordinatorOps::begin_funding_policy_rotation(begin.clone(), 11).expect("begin rotation");
    let after_begin = FleetCoordinatorFundingStore::export();
    FleetCoordinatorOps::begin_funding_policy_rotation(begin.clone(), 99)
        .expect("begin response-loss replay");
    assert_eq!(FleetCoordinatorFundingStore::export(), after_begin);
    FleetCoordinatorOps::set_root_funding_enabled(false)
        .expect_err("kill-switch mutation must not cross the rotation fence");

    for root in &plan.roots {
        let stage_root = FleetFundingPolicyRotationStageRootRequest {
            operation_id,
            plan_digest,
            root: root.clone(),
        };
        FleetCoordinatorOps::stage_funding_policy_rotation_root(stage_root.clone(), 12)
            .expect("stage Root");
        let staged = FleetCoordinatorFundingStore::export();
        FleetCoordinatorOps::stage_funding_policy_rotation_root(stage_root, 98)
            .expect("stage response-loss replay");
        assert_eq!(FleetCoordinatorFundingStore::export(), staged);
    }

    let apply = FleetFundingPolicyRotationApplyRequest {
        operation_id,
        plan_digest,
        expected_predecessor_generation: 1,
    };
    FleetCoordinatorOps::apply_funding_policy_rotation(apply.clone(), 13)
        .expect("apply fully staged plan");
    let applying = FleetCoordinatorFundingStore::export();
    FleetCoordinatorOps::apply_funding_policy_rotation(apply.clone(), 97)
        .expect("apply response-loss replay");
    assert_eq!(FleetCoordinatorFundingStore::export(), applying);

    for index in 0..plan.roots.len() {
        let FleetFundingPolicyRotationStep::PrepareRoot {
            fleet_subnet_root,
            request,
        } = FleetCoordinatorOps::funding_policy_rotation_step().expect("next Root preparation")
        else {
            panic!("rotation must prepare every Root before publication")
        };
        assert_eq!(fleet_subnet_root, plan.roots[index].fleet_subnet_root);
        FleetCoordinatorOps::record_funding_policy_rotation_root_prepared(
            rotation_root_receipt(&request, false, 20 + index as u64),
            20 + index as u64,
        )
        .expect("record prepared Root");
    }
    assert!(matches!(
        FleetCoordinatorOps::funding_policy_rotation_step().expect("publication step"),
        FleetFundingPolicyRotationStep::PublishRegistry
    ));
    let successor_registry = FleetCoordinatorOps::publish_funding_policy_rotation_registry(30)
        .expect("publish exact successor Registry");
    assert_eq!(
        successor_registry.revision,
        predecessor_registry.revision + 1
    );

    for index in 0..plan.roots.len() {
        let FleetFundingPolicyRotationStep::ActivateRoot {
            fleet_subnet_root,
            request,
        } = FleetCoordinatorOps::funding_policy_rotation_step().expect("next Root activation")
        else {
            panic!("rotation must activate every prepared Root")
        };
        assert_eq!(fleet_subnet_root, plan.roots[index].fleet_subnet_root);
        let receipt = FleetFundingPolicyRotationRootReceipt {
            operation_id: request.operation_id,
            plan_digest: request.plan_digest,
            fleet_subnet_root: request.fleet_subnet_root,
            predecessor_generation: request.predecessor_generation,
            successor_generation: request.successor_generation,
            prepared: true,
            activated: true,
            recorded_at_ns: 40 + index as u64,
        };
        FleetCoordinatorOps::record_funding_policy_rotation_root_activated(
            receipt,
            40 + index as u64,
        )
        .expect("record activated Root");
    }
    assert!(matches!(
        FleetCoordinatorOps::funding_policy_rotation_step().expect("completion step"),
        FleetFundingPolicyRotationStep::Complete
    ));
    let terminal =
        FleetCoordinatorOps::complete_funding_policy_rotation(50).expect("complete rotation");
    assert_eq!(terminal.operation_id, operation_id);
    assert_eq!(terminal.apply_operator_debit.to_u128(), 0);

    let completed = FleetCoordinatorFundingStore::export();
    FleetCoordinatorOps::apply_funding_policy_rotation(apply.clone(), 96)
        .expect("terminal apply replay");
    FleetCoordinatorOps::begin_funding_policy_rotation(begin.clone(), 95)
        .expect("terminal begin replay");
    FleetCoordinatorOps::stage_funding_policy_rotation_root(
        FleetFundingPolicyRotationStageRootRequest {
            operation_id,
            plan_digest,
            root: plan.roots[0].clone(),
        },
        94,
    )
    .expect("terminal stage replay");
    let mut drifted_begin = begin.clone();
    drifted_begin.header.topology_catalog_digest[0] ^= 1;
    FleetCoordinatorOps::begin_funding_policy_rotation(drifted_begin, 93)
        .expect_err("terminal begin replay must reject payload drift");
    let mut drifted_stage_root = plan.roots[0].clone();
    drifted_stage_root.placement.node_count += 1;
    FleetCoordinatorOps::stage_funding_policy_rotation_root(
        FleetFundingPolicyRotationStageRootRequest {
            operation_id,
            plan_digest,
            root: drifted_stage_root,
        },
        92,
    )
    .expect_err("terminal stage replay must reject payload drift");
    assert_eq!(FleetCoordinatorFundingStore::export(), completed);
    assert!(
        poll_ready(advance_funding_policy_rotation_once(operation_id))
            .expect("terminal scheduled replay stops")
    );
    let mut corrupted_checkpoint = completed.clone();
    corrupted_checkpoint
        .current
        .as_mut()
        .expect("terminal funding state")
        .rotation_history[0]
        .receipt
        .successor_policy_set_hash[0] ^= 1;
    FleetCoordinatorFundingStore::import(corrupted_checkpoint);
    FleetCoordinatorWorkflow::registry()
        .expect_err("corrupted funding-policy checkpoint must invalidate Registry recovery");
    FleetCoordinatorFundingStore::import(completed.clone());

    let funding = completed.current.expect("terminal funding state");
    assert_eq!(funding.policy_generation, 2);
    assert_eq!(
        funding.historical_automatic_grants,
        u64::from(first.funding.root_funding.maximum_automatic_grants)
    );
    assert_eq!(
        funding.historical_automatic_cycles,
        first.funding.root_funding.maximum_automatic_cycles
    );
    assert_eq!(funding.automatic_grants, 0);
    assert_eq!(funding.automatic_cycles.to_u128(), 0);
    assert!(funding.rotation_current.is_none());
    assert_eq!(funding.rotation_last, Some(terminal.clone()));

    let application_after = FleetCoordinatorWorkflow::registry().expect("successor Registry");
    assert_eq!(
        application_after.component_specs,
        application_before.component_specs
    );
    for (before, after) in application_before
        .fleet_subnet_roots
        .iter()
        .zip(&application_after.fleet_subnet_roots)
    {
        assert_eq!(after.component_admissions, before.component_admissions);
        assert_eq!(after.active_release_set, before.active_release_set);
        assert_eq!(after.limits, before.limits);
        assert_eq!(after.status, before.status);
    }

    let coordinator_policy = FleetCoordinatorRegistryStore::export()
        .current
        .expect("successor Coordinator state")
        .root_funding
        .expect("successor Coordinator policy");
    let mut second_plan = funding_rotation_plan(
        terminal.successor_registry,
        [
            &application_after.fleet_subnet_roots[0],
            &application_after.fleet_subnet_roots[1],
        ],
    );
    second_plan.header.predecessor_generation = 2;
    second_plan.header.successor_generation = 3;
    second_plan.header.predecessor_coordinator_policy_hash =
        coordinator_root_funding_policy_hash(&coordinator_policy);
    second_plan.header.predecessor_usage = FleetFundingPolicyUsage {
        historical_automatic_grants: funding.historical_automatic_grants,
        historical_automatic_cycles: funding.historical_automatic_cycles.clone(),
        generation_automatic_grants: funding.automatic_grants,
        generation_automatic_cycles: funding.automatic_cycles.clone(),
    };
    for root in &mut second_plan.roots {
        let ledger = funding
            .roots
            .iter()
            .find(|ledger| ledger.fleet_subnet_root == root.fleet_subnet_root);
        root.predecessor_usage = FleetFundingPolicyUsage {
            historical_automatic_grants: ledger
                .map_or(0, |ledger| ledger.historical_automatic_grants),
            historical_automatic_cycles: ledger.map_or_else(
                || Cycles::new(0),
                |ledger| ledger.historical_automatic_cycles.clone(),
            ),
            generation_automatic_grants: ledger.map_or(0, |ledger| ledger.automatic_grants),
            generation_automatic_cycles: ledger
                .map_or_else(|| Cycles::new(0), |ledger| ledger.automatic_cycles.clone()),
        };
    }
    second_plan.header.roots_digest =
        fleet_funding_policy_rotation_roots_digest(&second_plan.roots);
    let second_terminal = complete_policy_rotation_for_test(coordinator, second_plan, 100);
    let after_second_rotation =
        FleetCoordinatorWorkflow::registry().expect("second successor Registry remains canonical");
    assert_eq!(
        after_second_rotation.component_specs,
        application_before.component_specs
    );
    assert_eq!(
        FleetCoordinatorFundingStore::export()
            .current
            .expect("second terminal funding state")
            .rotation_history
            .len(),
        2
    );
    FleetCoordinatorOps::begin_funding_policy_rotation(begin, 121)
        .expect("older terminal begin remains replayable");
    FleetCoordinatorOps::stage_funding_policy_rotation_root(
        FleetFundingPolicyRotationStageRootRequest {
            operation_id,
            plan_digest,
            root: plan.roots[0].clone(),
        },
        122,
    )
    .expect("older terminal stage remains replayable");
    FleetCoordinatorOps::apply_funding_policy_rotation(apply, 123)
        .expect("older terminal apply remains replayable");
    assert!(matches!(
        FleetCoordinatorOps::funding_policy_rotation_status(operation_id)
            .expect("older terminal status")
            .expect("retained terminal operation")
            .phase,
        FleetFundingPolicyRotationStatusPhase::Completed(_)
    ));

    let successor_root = after_second_rotation
        .fleet_subnet_roots
        .iter()
        .find(|root| root.fleet_subnet_root == first.fleet_subnet_root)
        .expect("successor Root authority");
    let resumed_request = root_funding_request(
        coordinator,
        successor_root,
        second_terminal.successor_registry.clone(),
        u64::from(first.funding.root_funding.maximum_automatic_grants) + 1,
        0,
    );
    let resumed_at_ns = 6_u64
        .checked_mul(window_ns)
        .expect("successor grant observation time");
    let resumed_call = match FleetCoordinatorOps::prepare_root_funding(
        first.fleet_subnet_root,
        resumed_request,
        2_000_000_000_000_000,
        resumed_at_ns,
    )
    .expect("successor generation resumes automatic funding")
    {
        FleetRootFundingDisposition::Invoke(call) => call,
        FleetRootFundingDisposition::Current(_) | FleetRootFundingDisposition::Reconcile(_) => {
            panic!("fresh successor request must invoke")
        }
    };
    let resumed_receipt = FleetRootFundingAcceptanceReceipt {
        request: resumed_call.request.clone(),
        fleet_subnet_root: first.fleet_subnet_root,
        coordinator,
        accepted_at_ns: resumed_at_ns + 1,
    };
    FleetCoordinatorOps::record_root_funding_acceptance(
        first.fleet_subnet_root,
        &resumed_call.request,
        resumed_receipt,
        resumed_at_ns + 2,
    )
    .expect("commit successor generation grant");

    FleetCoordinatorWorkflow::publish_root_draining(root_draining_publication_request(
        successor_root,
        &second_terminal.successor_registry,
        [70; 32],
    ))
    .expect("application lifecycle remains writable after funding rotation");
}

fn funding_rotation_plan(
    predecessor_registry: FleetRegistryVersion,
    roots: [&FleetSubnetRootEntry; 2],
) -> FleetFundingPolicyRotationPlan {
    let mut proposed_coordinator = crate::test_support::coordinator_root_funding_policy();
    proposed_coordinator.funding_profile = FleetFundingProfile::PreviewMultiSubnet;
    proposed_coordinator.minimum_reserve_cycles = Cycles::new(80_000_000_000_000);
    proposed_coordinator.maximum_automatic_grants = 8;
    proposed_coordinator.maximum_automatic_cycles = Cycles::new(240_000_000_000_000);
    let maximum_new_automatic_cycles = proposed_coordinator.maximum_automatic_cycles.clone();
    let zero_usage = FleetFundingPolicyUsage {
        historical_automatic_grants: 0,
        historical_automatic_cycles: Cycles::new(0),
        generation_automatic_grants: 0,
        generation_automatic_cycles: Cycles::new(0),
    };
    let mut root_plans = roots
        .into_iter()
        .map(|root| {
            let mut proposed_policy = root.funding.root_funding.clone();
            proposed_policy.funding_profile = FleetFundingProfile::PreviewMultiSubnet;
            FleetFundingPolicyRotationRootPlan {
                fleet_subnet_root: root.fleet_subnet_root,
                predecessor_policy_hash: fleet_subnet_root_funding_policy_hash(&root.funding),
                predecessor_usage: zero_usage.clone(),
                proposed_policy,
                placement: FleetFundingPolicyRotationPlacementEvidence {
                    subnet: root.placement_subnet,
                    node_count: 13,
                    cost_multiplier_numerator: 1,
                    cost_multiplier_denominator: 1,
                    fiduciary: false,
                    acknowledge_fiduciary_cost: false,
                },
            }
        })
        .collect::<Vec<_>>();
    root_plans.sort_by_key(|root| root.fleet_subnet_root);
    let mut plan = FleetFundingPolicyRotationPlan {
        header: FleetFundingPolicyRotationPlanHeader {
            predecessor_registry,
            predecessor_generation: 1,
            successor_generation: 2,
            predecessor_coordinator_policy_hash: coordinator_root_funding_policy_hash(
                &crate::test_support::coordinator_root_funding_policy(),
            ),
            predecessor_usage: zero_usage,
            proposed_coordinator_policy: proposed_coordinator,
            topology_catalog_digest: [69; 32],
            coordinator_placement: FleetFundingPolicyRotationPlacementEvidence {
                subnet: SubnetId::from_principal(principal(2)),
                node_count: 13,
                cost_multiplier_numerator: 1,
                cost_multiplier_denominator: 1,
                fiduciary: false,
                acknowledge_fiduciary_cost: false,
            },
            affected_root_count: 2,
            roots_digest: [0; 32],
            maximum_new_automatic_cycles,
            apply_operator_debit: Cycles::new(0),
            funding_source: FleetFundingPolicyRotationFundingSource::CoordinatorTreasury,
        },
        roots: root_plans,
    };
    plan.header.roots_digest = fleet_funding_policy_rotation_roots_digest(&plan.roots);
    plan
}

fn rotation_root_receipt(
    request: &canic_core::dto::fleet_funding::FleetFundingPolicyRotationRootPrepareRequest,
    activated: bool,
    recorded_at_ns: u64,
) -> FleetFundingPolicyRotationRootReceipt {
    FleetFundingPolicyRotationRootReceipt {
        operation_id: request.operation_id,
        plan_digest: request.plan_digest,
        fleet_subnet_root: request.root.fleet_subnet_root,
        predecessor_generation: request.predecessor_generation,
        successor_generation: request.successor_generation,
        prepared: true,
        activated,
        recorded_at_ns,
    }
}

fn complete_policy_rotation_for_test(
    coordinator: Principal,
    plan: FleetFundingPolicyRotationPlan,
    timestamp_base: u64,
) -> canic_core::dto::fleet_funding::FleetFundingPolicyRotationReceipt {
    let plan_digest = fleet_funding_policy_rotation_plan_digest(&plan);
    let operation_id = fleet_funding_policy_rotation_operation_id(coordinator, plan_digest);
    FleetCoordinatorOps::begin_funding_policy_rotation(
        FleetFundingPolicyRotationBeginRequest {
            operation_id,
            plan_digest,
            header: plan.header.clone(),
        },
        timestamp_base,
    )
    .expect("begin successor rotation");
    for root in &plan.roots {
        FleetCoordinatorOps::stage_funding_policy_rotation_root(
            FleetFundingPolicyRotationStageRootRequest {
                operation_id,
                plan_digest,
                root: root.clone(),
            },
            timestamp_base + 1,
        )
        .expect("stage successor Root");
    }
    FleetCoordinatorOps::apply_funding_policy_rotation(
        FleetFundingPolicyRotationApplyRequest {
            operation_id,
            plan_digest,
            expected_predecessor_generation: plan.header.predecessor_generation,
        },
        timestamp_base + 2,
    )
    .expect("apply successor rotation");
    for index in 0..plan.roots.len() {
        let FleetFundingPolicyRotationStep::PrepareRoot { request, .. } =
            FleetCoordinatorOps::funding_policy_rotation_step()
                .expect("successor Root preparation")
        else {
            panic!("successor rotation must prepare every Root")
        };
        FleetCoordinatorOps::record_funding_policy_rotation_root_prepared(
            rotation_root_receipt(&request, false, timestamp_base + 3 + index as u64),
            timestamp_base + 3 + index as u64,
        )
        .expect("record successor Root preparation");
    }
    FleetCoordinatorOps::publish_funding_policy_rotation_registry(timestamp_base + 10)
        .expect("publish successor rotation Registry");
    for index in 0..plan.roots.len() {
        let FleetFundingPolicyRotationStep::ActivateRoot { request, .. } =
            FleetCoordinatorOps::funding_policy_rotation_step().expect("successor Root activation")
        else {
            panic!("successor rotation must activate every Root")
        };
        FleetCoordinatorOps::record_funding_policy_rotation_root_activated(
            FleetFundingPolicyRotationRootReceipt {
                operation_id: request.operation_id,
                plan_digest: request.plan_digest,
                fleet_subnet_root: request.fleet_subnet_root,
                predecessor_generation: request.predecessor_generation,
                successor_generation: request.successor_generation,
                prepared: true,
                activated: true,
                recorded_at_ns: timestamp_base + 11 + index as u64,
            },
            timestamp_base + 11 + index as u64,
        )
        .expect("record successor Root activation");
    }
    FleetCoordinatorOps::complete_funding_policy_rotation(timestamp_base + 20)
        .expect("complete successor rotation")
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one end-to-end durable grant and response-loss proof"
)]
fn coordinator_root_funding_reserves_replays_and_commits_one_exact_grant() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let coordinator = principal(70);
    let (root, _, registry) = activate_two_roots(coordinator);
    let now_ns = 3_700_000_000_000;
    let coordinator_balance = 2_000_000_000_000_000;
    let request = root_funding_request(coordinator, &root, registry, 1, 42_200_000_000);
    let granted_cycles = request.requested_cycles.to_u128();

    let call = match FleetCoordinatorOps::prepare_root_funding(
        root.fleet_subnet_root,
        request.clone(),
        coordinator_balance,
        now_ns,
    )
    .expect("prepare exact grant")
    {
        FleetRootFundingDisposition::Invoke(call) => call,
        FleetRootFundingDisposition::Current(_) | FleetRootFundingDisposition::Reconcile(_) => {
            panic!("fresh request must create one invocation")
        }
    };
    assert_eq!(call.fleet_subnet_root, root.fleet_subnet_root);
    assert_eq!(call.request.granted_cycles.to_u128(), granted_cycles);
    let funding_status = FleetCoordinatorOps::root_funding_status(coordinator_balance, now_ns)
        .expect("protected Coordinator funding status");
    assert!(funding_status.funding_enabled);
    assert_eq!(funding_status.current_cycles.to_u128(), coordinator_balance);
    assert_eq!(funding_status.roots.len(), 2);
    assert_eq!(
        funding_status
            .fleet_window
            .as_ref()
            .expect("Fleet window")
            .reserved_cycles
            .to_u128(),
        granted_cycles
    );
    let root_status = funding_status
        .roots
        .iter()
        .find(|status| status.fleet_subnet_root == root.fleet_subnet_root)
        .expect("requesting Root status");
    assert_eq!(
        root_status
            .current_operation
            .as_ref()
            .expect("current funding operation"),
        &request
    );
    assert_eq!(root_status.window.reserved_cycles.to_u128(), granted_cycles);
    FleetCoordinatorOps::require_root_funding_snapshot_resumable()
        .expect_err("unresolved attached-cycles grant must fence Coordinator snapshot");

    let reserved = FleetCoordinatorFundingStore::export();
    let ledger = &reserved.current.as_ref().expect("funding state").roots[0];
    assert_eq!(
        ledger.current.as_ref().expect("active intent").request,
        request
    );
    assert!(ledger.last.is_none());

    assert!(matches!(
        FleetCoordinatorOps::prepare_root_funding(
            root.fleet_subnet_root,
            request.clone(),
            coordinator_balance,
            now_ns + 1,
        )
        .expect("exact active retry"),
        FleetRootFundingDisposition::Reconcile(_)
    ));

    let competing = root_funding_request(
        coordinator,
        &root,
        request.expected_registry.clone(),
        1,
        42_200_000_001,
    );
    assert!(
        FleetCoordinatorOps::prepare_root_funding(
            root.fleet_subnet_root,
            competing,
            coordinator_balance,
            now_ns + 2,
        )
        .is_err()
    );

    let receipt = FleetRootFundingAcceptanceReceipt {
        request: call.request.clone(),
        fleet_subnet_root: root.fleet_subnet_root,
        coordinator,
        accepted_at_ns: now_ns + 3,
    };
    let response = FleetCoordinatorOps::record_root_funding_acceptance(
        root.fleet_subnet_root,
        &call.request,
        receipt,
        now_ns + 4,
    )
    .expect("commit exact acceptance");
    assert!(matches!(&response, FleetRootFundingResponse::Granted(_)));

    let completed = FleetCoordinatorFundingStore::export();
    FleetCoordinatorFundingStore::import(completed.clone());
    let completed_record = completed.current.as_ref().expect("completed funding state");
    assert_eq!(completed_record.automatic_grants, 1);
    assert_eq!(completed_record.automatic_cycles.to_u128(), granted_cycles);
    assert_eq!(
        completed_record
            .fleet_window
            .as_ref()
            .expect("Fleet spend")
            .spent_cycles
            .to_u128(),
        granted_cycles
    );
    let ledger = &completed_record.roots[0];
    assert_eq!(ledger.automatic_grants, 1);
    assert_eq!(ledger.automatic_cycles.to_u128(), granted_cycles);
    assert!(ledger.current.is_none());
    assert_eq!(
        ledger
            .window
            .as_ref()
            .expect("Root spend")
            .spent_cycles
            .to_u128(),
        granted_cycles
    );
    assert_eq!(ledger.last_successful_grant_at_ns, Some(now_ns + 3));
    FleetCoordinatorOps::require_root_funding_snapshot_resumable()
        .expect("terminal grant permits Coordinator snapshot");

    match FleetCoordinatorOps::prepare_root_funding(
        root.fleet_subnet_root,
        request,
        coordinator_balance,
        now_ns + 5,
    )
    .expect("replay terminal result")
    {
        FleetRootFundingDisposition::Current(replayed) => assert_eq!(replayed, response),
        FleetRootFundingDisposition::Invoke(_) | FleetRootFundingDisposition::Reconcile(_) => {
            panic!("terminal retry must not create another transfer")
        }
    }
    assert_eq!(FleetCoordinatorFundingStore::export(), completed);
    assert!(
        FleetCoordinatorOps::record_root_funding_acceptance(
            root.fleet_subnet_root,
            &call.request,
            match response {
                FleetRootFundingResponse::Granted(receipt) => receipt,
                FleetRootFundingResponse::NoGrant(_) => panic!("expected granted receipt"),
            },
            now_ns + 6,
        )
        .is_err()
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one end-to-end terminal denial and rejection proof"
)]
fn coordinator_root_funding_denials_and_rejections_are_terminal_without_spend() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let coordinator = principal(71);
    let (root, _, registry) = activate_two_roots(coordinator);
    let now_ns = 3_700_000_000_000;
    let coordinator_balance = 2_000_000_000_000_000;

    assert_eq!(
        FleetCoordinatorOps::set_root_funding_enabled(false).expect("disable funding"),
        canic_core::dto::state::SetStateResponse {
            previous: true,
            current: false,
            changed: true,
        }
    );
    let disabled = root_funding_request(coordinator, &root, registry.clone(), 1, 42_200_000_000);
    let disabled_response = match FleetCoordinatorOps::prepare_root_funding(
        root.fleet_subnet_root,
        disabled,
        coordinator_balance,
        now_ns,
    )
    .expect("disabled decision")
    {
        FleetRootFundingDisposition::Current(response) => response,
        FleetRootFundingDisposition::Invoke(_) | FleetRootFundingDisposition::Reconcile(_) => {
            panic!("disabled funding cannot reserve")
        }
    };
    assert!(matches!(
        disabled_response,
        FleetRootFundingResponse::NoGrant(ref receipt)
            if receipt.reason == FleetRootFundingNoGrantReason::FundingDisabled
    ));

    FleetCoordinatorOps::set_root_funding_enabled(true).expect("enable funding");
    let durable_before_substitution = FleetCoordinatorFundingStore::export();
    let mut substituted_authority =
        root_funding_request(coordinator, &root, registry.clone(), 2, 42_200_000_000);
    substituted_authority
        .expected_registry
        .authority
        .binding
        .fleet
        .app = AppId::from("substituted");
    substituted_authority.operation_id = fleet_root_funding_operation_id(
        coordinator,
        root.fleet_subnet_root,
        substituted_authority.operation_sequence,
        &substituted_authority.expected_registry,
        substituted_authority.observed_balance.to_u128(),
        substituted_authority.requested_cycles.to_u128(),
        substituted_authority.policy_hash,
    );
    let Err(authority_error) = FleetCoordinatorOps::prepare_root_funding(
        root.fleet_subnet_root,
        substituted_authority,
        coordinator_balance,
        now_ns + 1,
    ) else {
        panic!("substituted Registry authority must reject before retention");
    };
    assert_eq!(
        authority_error.public_error().code(),
        canic_core::diagnostics::codes::REQUEST_INVALID.raw_code()
    );
    assert_eq!(
        FleetCoordinatorFundingStore::export(),
        durable_before_substitution
    );

    let mut mismatched =
        root_funding_request(coordinator, &root, registry.clone(), 2, 42_200_000_000);
    mismatched.policy_hash = [99; 32];
    mismatched.operation_id = fleet_root_funding_operation_id(
        coordinator,
        root.fleet_subnet_root,
        mismatched.operation_sequence,
        &mismatched.expected_registry,
        mismatched.observed_balance.to_u128(),
        mismatched.requested_cycles.to_u128(),
        mismatched.policy_hash,
    );
    assert!(matches!(
        FleetCoordinatorOps::prepare_root_funding(
            root.fleet_subnet_root,
            mismatched,
            coordinator_balance,
            now_ns + 2,
        )
        .expect("retain policy mismatch denial"),
        FleetRootFundingDisposition::Current(FleetRootFundingResponse::NoGrant(receipt))
            if receipt.reason == FleetRootFundingNoGrantReason::PolicyMismatch
    ));

    let request = root_funding_request(coordinator, &root, registry, 3, 42_200_000_000);
    let call = match FleetCoordinatorOps::prepare_root_funding(
        root.fleet_subnet_root,
        request.clone(),
        coordinator_balance,
        now_ns + 3,
    )
    .expect("prepare successor")
    {
        FleetRootFundingDisposition::Invoke(call) => call,
        FleetRootFundingDisposition::Current(_) | FleetRootFundingDisposition::Reconcile(_) => {
            panic!("valid successor must reserve")
        }
    };
    let rejected = FleetCoordinatorOps::record_root_funding_rejection(
        root.fleet_subnet_root,
        &call.request,
        now_ns + 4,
    )
    .expect("record definite rejection");
    assert!(matches!(
        rejected,
        FleetRootFundingResponse::NoGrant(ref receipt)
            if receipt.reason == FleetRootFundingNoGrantReason::RootRejected
    ));

    let durable = FleetCoordinatorFundingStore::export();
    let durable = durable.current.expect("funding state");
    assert!(durable.fleet_window.is_none());
    assert!(durable.roots[0].window.is_none());
    assert!(durable.roots[0].current.is_none());
    assert!(matches!(
        FleetCoordinatorOps::prepare_root_funding(
            root.fleet_subnet_root,
            request,
            coordinator_balance,
            now_ns + 5,
        )
        .expect("replay rejection"),
        FleetRootFundingDisposition::Current(response) if response == rejected
    ));

    assert!(FleetCoordinatorOps::authorize_root_funding_caller(Principal::anonymous()).is_err());
    assert!(FleetCoordinatorOps::authorize_root_funding_caller(principal(99)).is_err());
    assert!(FleetCoordinatorOps::authorize_root_funding_caller(coordinator).is_err());
}

fn root_funding_request(
    coordinator: Principal,
    root: &FleetSubnetRootEntry,
    expected_registry: canic_core::dto::fleet_registry::FleetRegistryVersion,
    operation_sequence: u64,
    observed_balance: u128,
) -> FleetRootFundingRequest {
    let policy_hash = fleet_subnet_root_funding_policy_hash(&root.funding);
    let requested_cycles = root
        .funding
        .root_funding
        .target_balance
        .to_u128()
        .checked_sub(observed_balance)
        .expect("fixture target exceeds observation");
    FleetRootFundingRequest {
        operation_id: fleet_root_funding_operation_id(
            coordinator,
            root.fleet_subnet_root,
            operation_sequence,
            &expected_registry,
            observed_balance,
            requested_cycles,
            policy_hash,
        ),
        operation_sequence,
        expected_registry,
        observed_balance: Cycles::new(observed_balance),
        requested_cycles: Cycles::new(requested_cycles),
        policy_hash,
    }
}
