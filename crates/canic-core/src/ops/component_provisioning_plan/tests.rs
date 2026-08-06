//! Focused proofs for canonical root/placement/member provisioning plans.

use super::*;
use crate::{
    bootstrap::parse_config_model,
    dto::{
        component_provisioning::{
            ComponentGroupPlacementPlan, ComponentGroupPlanEntry,
            FleetComponentProvisioningOperation, FleetComponentProvisioningPlan,
            FleetSubnetRootProvisioningBatch, RootComponentProvisioningAcceptanceRequest,
            RootComponentProvisioningAdvanceRequest, RootComponentProvisioningPhase,
            RootComponentProvisioningStatusRequest, RootComponentProvisioningStatusResponse,
        },
        fleet_registry::{FleetRegistry, FleetSubnetRootEntry, FleetSubnetRootStatus},
    },
    ids::{
        AppId, CanonicalNetworkId, ComponentGroupPlacementId, ComponentSpecAdmission,
        CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
        FleetRegistryAuthority, FleetSubnetCanisterPoolConfig, FleetSubnetRootBinding,
        FleetSubnetRootLimits, FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce,
        ReleaseSetDigest, SubnetId,
    },
};

const CONFIG: &str = r#"
[app]
name = "plan_test"

[roles.root]
kind = "root"
package = "root"

[roles.alpha]
kind = "canister"
package = "alpha"

[roles.beta]
kind = "canister"
package = "beta"

[component_specs.alpha]
component_role = "alpha"
maximum_instances = 4

[component_specs.beta]
component_role = "beta"
maximum_instances = 4

[component_groups.cell.components.alpha]
component_spec = "alpha"
labels = { tier = "api" }

[component_groups.cell.components.beta]
component_spec = "beta"

[component_group_deployments.cells]
component_group = "cell"
initial_placements = 2
maximum_placements = 4
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 2
"#;

const SERVICE_CONFIG: &str = r#"
[app]
name = "plan_test"

[roles.root]
kind = "root"
package = "root"

[roles.alpha]
kind = "canister"
package = "alpha"

[component_specs.alpha]
component_role = "alpha"
maximum_instances = 4

[component_groups.cell.components.alpha]
component_spec = "alpha"
service = "api"

[component_group_deployments.cells]
component_group = "cell"
service_purpose = "pool_member"
initial_placements = 2
maximum_placements = 4
placement.maximum_per_root = 2
placement.minimum_distinct_roots = 1

[services.fleet.targets.api]
role = "alpha"
component_spec = "alpha"
mode = "active_pool"
placement.maximum_members_per_root = 1
placement.minimum_distinct_roots = 2
"#;

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
                    fleet_id: FleetId::from_generated_bytes([7; 32]),
                },
                app: AppId::from("plan_test"),
            },
            coordinator_subnet: subnet(2),
            coordinator: principal(3),
        },
        epoch: 1,
    }
}

fn release_set() -> FleetSubnetRootReleaseSet {
    FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([8; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
    }
}

fn limits(maximum_group_placements: u32) -> FleetSubnetRootLimits {
    FleetSubnetRootLimits {
        maximum_component_instances: 4,
        maximum_registry_bytes: 4_194_304,
        maximum_wasm_store_bytes: 40_000_000,
        canister_pool: FleetSubnetCanisterPoolConfig {
            minimum_size: 1,
            maximum_size: 10,
            canister_cycles: Cycles::new(5_000_000_000_000),
        },
        cycles_funding: CyclesFundingBudget {
            window_secs: 3_600,
            maximum_cycles: Cycles::new(10_000_000_000_000),
        },
        maximum_group_placements,
    }
}

fn root_entry(
    topology: &ComponentTopology,
    subnet_byte: u8,
    root_byte: u8,
    maximum_group_placements: u32,
) -> FleetSubnetRootEntry {
    let component_admissions = topology
        .component_specs
        .iter()
        .map(|spec| ComponentSpecAdmission {
            component_spec: spec.component_spec.clone(),
            spec_hash: spec.spec_hash,
            maximum_root_instances: 2,
        })
        .collect::<Vec<_>>();
    let projection = topology
        .project_for_admissions(&component_admissions)
        .expect("root topology projection");
    FleetSubnetRootEntry {
        placement_subnet: subnet(subnet_byte),
        fleet_subnet_root: principal(root_byte),
        component_admissions,
        component_topology_digest: projection.digest().expect("root topology digest"),
        active_release_set: release_set(),
        limits: limits(maximum_group_placements),
        status: FleetSubnetRootStatus::Joining,
    }
}

fn active_registry(config: &ConfigModel, maximum_group_placements: u32) -> FleetRegistry {
    let topology = config
        .compile_component_topology()
        .expect("Component Topology");
    let authority = authority();
    let mut registry = FleetRegistryOps::compile_genesis(
        &authority.binding.fleet.app,
        authority.clone(),
        &topology,
    )
    .expect("genesis");
    for root in [
        root_entry(&topology, 4, 5, maximum_group_placements),
        root_entry(&topology, 6, 7, maximum_group_placements),
    ] {
        registry = FleetRegistryOps::compile_joining(&authority, &topology, &registry, root)
            .expect("join root");
    }
    FleetRegistryOps::compile_active(&authority, &topology, &registry).expect("activate roots")
}

fn binding(registry: &FleetRegistry, root: &FleetSubnetRootEntry) -> FleetSubnetRootBinding {
    FleetSubnetRootBinding {
        authority: registry.authority.clone(),
        placement_subnet: root.placement_subnet,
        fleet_subnet_root: root.fleet_subnet_root,
        component_admissions: root.component_admissions.clone(),
        component_topology_digest: root.component_topology_digest,
        limits: root.limits.clone(),
    }
}

fn plan(config: &ConfigModel, registry: &FleetRegistry) -> FleetComponentProvisioningPlan {
    let topology = config
        .compile_component_topology()
        .expect("Component Topology");
    let deployment_topology = config
        .compile_component_group_deployment_topology()
        .expect("deployment topology");
    let deployment = deployment_topology
        .get(&"cells".parse().expect("deployment ID"))
        .expect("cells deployment");
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
            root: binding(registry, root),
            active_release_set: root.active_release_set,
            placements: vec![ComponentGroupPlacementPlan {
                group_placement: ComponentGroupPlacementId {
                    deployment: deployment.deployment.clone(),
                    ordinal: u32::try_from(ordinal).expect("bounded ordinal"),
                },
                component_group: deployment.component_group.clone(),
                entries: entries.clone(),
            }],
        })
        .collect::<Vec<_>>();
    let mut confirmation_roots = registry
        .fleet_subnet_roots
        .iter()
        .map(|root| root.fleet_subnet_root)
        .collect::<Vec<_>>();
    confirmation_roots.sort_unstable();
    FleetComponentProvisioningPlan {
        fleet: registry.authority.binding.fleet.clone(),
        fleet_registry: FleetRegistryOps::version(&registry.authority, &topology, registry)
            .expect("Registry version"),
        configuration_digest: config
            .compile_component_deployment_configuration_digest()
            .expect("configuration digest"),
        operation: FleetComponentProvisioningOperation::FreshInstall,
        directory_confirmation_roots: confirmation_roots,
        batches,
    }
}

fn fixture(
    maximum_group_placements: u32,
) -> (ConfigModel, FleetRegistry, FleetComponentProvisioningPlan) {
    fixture_from(CONFIG, maximum_group_placements)
}

fn fixture_from(
    source: &str,
    maximum_group_placements: u32,
) -> (ConfigModel, FleetRegistry, FleetComponentProvisioningPlan) {
    let config = parse_config_model(source).expect("valid config");
    let registry = active_registry(&config, maximum_group_placements);
    let plan = plan(&config, &registry);
    (config, registry, plan)
}

#[test]
fn canonical_plan_binds_exact_roots_placements_members_and_limits() {
    let (config, registry, plan) = fixture(1);
    validate(&config, &registry, &plan).expect("valid plan");
    let bytes = canonical_bytes(&config, &registry, &plan).expect("canonical plan bytes");
    let hash: [u8; 32] = Sha256::digest(&bytes).into();

    assert_eq!(
        crate::ids::ComponentDeploymentConfigurationDigest::from_bytes(hash).to_string(),
        "bd57204ba85251e3b1c96659ad10c6af25a2c96eafe8139175dd55c69dd059e6"
    );

    assert_eq!(
        ComponentProvisioningPlanOps::hash(&config, &registry, &plan).expect("public plan hash"),
        hash
    );
    assert_eq!(
        candid::decode_one::<FleetComponentProvisioningPlan>(
            &candid::encode_one(&plan).expect("encode plan")
        )
        .expect("decode plan"),
        plan
    );
}

#[test]
fn canonical_plan_rejects_reordering_substitution_and_wrong_initial_ordinals() {
    let (config, registry, plan) = fixture(1);

    let mut reordered = plan.clone();
    reordered.batches.reverse();
    crate::assert_err_variant!(
        validate(&config, &registry, &reordered),
        Err(ComponentProvisioningPlanOpsError::NonCanonicalBatchOrder)
    );

    let mut substituted = plan.clone();
    substituted.batches[0].placements[0].entries[0]
        .limits
        .maximum_descendants += 1;
    crate::assert_err_variant!(
        validate(&config, &registry, &substituted),
        Err(ComponentProvisioningPlanOpsError::PlacementEntriesMismatch)
    );

    let mut skipped = plan;
    skipped.batches[1].placements[0].group_placement.ordinal = 3;
    crate::assert_err_variant!(
        validate(&config, &registry, &skipped),
        Err(ComponentProvisioningPlanOpsError::FreshInstallPlacementSetMismatch)
    );
}

#[test]
fn protected_root_group_placement_ceiling_rejects_before_plan_hashing() {
    let (config, registry, plan) = fixture(0);

    crate::assert_err_variant!(
        validate(&config, &registry, &plan),
        Err(ComponentProvisioningPlanOpsError::RootGroupPlacementCapacityExceeded)
    );
    assert!(ComponentProvisioningPlanOps::hash(&config, &registry, &plan).is_err());
}

#[test]
fn exact_root_batch_validation_returns_bounded_capacity_and_artifact_facts() {
    let (config, registry, plan) = fixture(1);
    let batch = &plan.batches[0];
    let validation = validate_root_batch(
        &config,
        &registry,
        &plan.fleet_registry,
        plan.configuration_digest,
        &batch.root,
        batch,
    )
    .expect("valid root batch");

    assert_eq!(validation.placement_count, 1);
    assert_eq!(validation.component_count, 2);
    assert_eq!(validation.component_spec_counts.len(), 2);
    assert_eq!(validation.component_roles.len(), 2);
    let bytes = ComponentProvisioningPlanOps::root_batch_canonical_bytes(
        &config,
        &registry,
        &plan.fleet_registry,
        plan.configuration_digest,
        &batch.root,
        batch,
    )
    .expect("canonical root batch");
    assert!(bytes.len() <= MAX_FLEET_SUBNET_ROOT_PROVISIONING_BATCH_CANONICAL_BYTES);
    let hash: [u8; 32] = Sha256::digest(bytes).into();
    assert_eq!(
        hash,
        [
            56, 62, 229, 20, 255, 138, 237, 163, 237, 11, 43, 6, 103, 35, 227, 70, 137, 144, 159,
            139, 238, 99, 22, 4, 149, 162, 16, 250, 30, 115, 132, 51,
        ]
    );

    let request = RootComponentProvisioningAcceptanceRequest {
        fleet_registry: plan.fleet_registry.clone(),
        configuration_digest: plan.configuration_digest,
        operation_id: [13; 32],
        plan_hash: [14; 32],
        batch: batch.clone(),
    };
    assert_eq!(
        candid::decode_one::<RootComponentProvisioningAcceptanceRequest>(
            &candid::encode_one(&request).expect("encode acceptance request")
        )
        .expect("decode acceptance request"),
        request
    );
    let status_request = RootComponentProvisioningStatusRequest {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
    };
    assert_eq!(
        candid::decode_one::<RootComponentProvisioningStatusRequest>(
            &candid::encode_one(status_request).expect("encode status request")
        )
        .expect("decode status request"),
        status_request
    );
    let advance_request = RootComponentProvisioningAdvanceRequest {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
        expected_reserved_component_count: 0,
        expected_claimed_component_count: 0,
        expected_installed_component_count: 0,
        expected_registry_committed_component_count: 0,
    };
    assert_eq!(
        candid::decode_one::<RootComponentProvisioningAdvanceRequest>(
            &candid::encode_one(advance_request).expect("encode advance request")
        )
        .expect("decode advance request"),
        advance_request
    );
    let response = RootComponentProvisioningStatusResponse {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
        fleet_registry: request.fleet_registry,
        configuration_digest: request.configuration_digest,
        fleet_subnet_root: request.batch.root.fleet_subnet_root,
        phase: RootComponentProvisioningPhase::Accepted,
        placement_count: validation.placement_count,
        component_count: validation.component_count,
        reserved_component_count: 0,
        claimed_component_count: 0,
        installed_component_count: 0,
        registry_committed_component_count: 0,
        accepted_at_ns: 1,
        receipt_content_hash: [15; 32],
    };
    assert_eq!(
        candid::decode_one::<RootComponentProvisioningStatusResponse>(
            &candid::encode_one(&response).expect("encode status response")
        )
        .expect("decode status response"),
        response
    );
}

#[test]
fn exact_root_batch_rejects_local_deployment_density_excess() {
    let (config, registry, plan) = fixture(2);
    let mut batch = plan.batches[0].clone();
    batch.placements.push(plan.batches[1].placements[0].clone());

    crate::assert_err_variant!(
        validate_root_batch(
            &config,
            &registry,
            &plan.fleet_registry,
            plan.configuration_digest,
            &batch.root,
            &batch,
        ),
        Err(ComponentProvisioningPlanOpsError::RootBatchDeploymentDensityExceeded)
    );
}

#[test]
fn deployment_placements_may_share_one_root_within_density_and_capacity() {
    let source = CONFIG.replace(
        "placement.maximum_per_root = 1\nplacement.minimum_distinct_roots = 2",
        "placement.maximum_per_root = 2\nplacement.minimum_distinct_roots = 1",
    );
    let (config, registry, mut plan) = fixture_from(&source, 2);
    let second = plan.batches.remove(1);
    plan.batches[0].placements.extend(second.placements);

    validate(&config, &registry, &plan).expect("valid packed deployment plan");
    validate_root_batch(
        &config,
        &registry,
        &plan.fleet_registry,
        plan.configuration_digest,
        &plan.batches[0].root,
        &plan.batches[0],
    )
    .expect("valid packed root batch");
}

#[test]
fn service_density_and_spread_are_independent_from_deployment_policy() {
    let (config, registry, plan) = fixture_from(SERVICE_CONFIG, 2);
    validate(&config, &registry, &plan).expect("distributed service plan");

    let mut concentrated = plan.clone();
    let second = concentrated.batches.remove(1);
    concentrated.batches[0].placements.extend(second.placements);
    crate::assert_err_variant!(
        validate(&config, &registry, &concentrated),
        Err(ComponentProvisioningPlanOpsError::FreshInstallServicePlacementPolicyMismatch)
    );
    crate::assert_err_variant!(
        validate_root_batch(
            &config,
            &registry,
            &plan.fleet_registry,
            plan.configuration_digest,
            &concentrated.batches[0].root,
            &concentrated.batches[0],
        ),
        Err(ComponentProvisioningPlanOpsError::RootBatchServiceDensityExceeded)
    );
}

#[test]
fn scale_out_stays_fenced_until_durable_placement_state_is_authoritative() {
    let (config, registry, mut plan) = fixture(1);
    plan.operation = FleetComponentProvisioningOperation::ScaleOut {
        deployment: "cells".parse().expect("deployment ID"),
        previous_placements: 2,
        requested_placements: 4,
    };

    crate::assert_err_variant!(
        validate(&config, &registry, &plan),
        Err(ComponentProvisioningPlanOpsError::ScaleOutStateUnavailable)
    );
}

#[test]
fn plan_count_bounds_reject_the_first_excess_before_identity_validation() {
    let (config, registry, plan) = fixture(1);

    let mut excessive_batches = plan.clone();
    excessive_batches.batches =
        vec![plan.batches[0].clone(); MAX_FLEET_COMPONENT_PROVISIONING_PLAN_BATCHES + 1];
    crate::assert_err_variant!(
        validate(&config, &registry, &excessive_batches),
        Err(ComponentProvisioningPlanOpsError::BatchBoundExceeded {
            actual,
            maximum,
        }) if actual == maximum + 1
            && maximum == MAX_FLEET_COMPONENT_PROVISIONING_PLAN_BATCHES
    );

    let mut excessive_placements = plan.clone();
    excessive_placements.batches.truncate(1);
    excessive_placements.batches[0].placements = vec![
        plan.batches[0].placements[0].clone();
        MAX_FLEET_COMPONENT_PROVISIONING_PLAN_PLACEMENTS
            + 1
    ];
    crate::assert_err_variant!(
        validate(&config, &registry, &excessive_placements),
        Err(ComponentProvisioningPlanOpsError::PlacementBoundExceeded {
            actual,
            maximum,
        }) if actual == maximum + 1
            && maximum == MAX_FLEET_COMPONENT_PROVISIONING_PLAN_PLACEMENTS
    );

    let mut excessive_entries = plan;
    excessive_entries.batches.truncate(1);
    let entry = excessive_entries.batches[0].placements[0].entries[0].clone();
    excessive_entries.batches[0].placements[0].entries =
        vec![entry; MAX_FLEET_COMPONENT_PROVISIONING_PLAN_ENTRIES + 1];
    crate::assert_err_variant!(
        validate(&config, &registry, &excessive_entries),
        Err(ComponentProvisioningPlanOpsError::EntryBoundExceeded {
            actual,
            maximum,
        }) if actual == maximum + 1
            && maximum == MAX_FLEET_COMPONENT_PROVISIONING_PLAN_ENTRIES
    );
}
