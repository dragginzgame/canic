//! Focused proofs for receipt-derived Fleet-service bindings.

use super::*;
use crate::{
    bootstrap::parse_config_model,
    dto::{
        component_provisioning::{
            ComponentGroupPlacementPlan, ComponentGroupPlanEntry,
            FleetComponentProvisioningOperation, FleetComponentProvisioningPlan,
            FleetSubnetRootProvisioningBatch, RootComponentProvisioningResult,
            RootComponentProvisioningStatusResponse, RootProvisionedGroupMember,
            RootProvisionedGroupPlacement,
        },
        cycles::Cycles,
        fleet_registry::{FleetRegistry, FleetSubnetRootEntry, FleetSubnetRootStatus},
    },
    ids::{
        AppId, CanonicalNetworkId, ComponentBinding, ComponentGroupPlacementId,
        ComponentInstanceId, ComponentSpecAdmission, CyclesFundingBudget, FleetBinding,
        FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority,
        FleetSubnetCanisterPoolConfig, FleetSubnetRootBinding, FleetSubnetRootLimits,
        FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    },
    ops::{
        component_provisioning_plan::{
            ComponentProvisioningPlacementAuthority, ComponentProvisioningPlanOps,
            ComponentProvisioningScaleOutAuthority,
        },
        component_provisioning_receipt::{
            RootComponentProvisioningProvisionedReceiptAuthority,
            RootComponentProvisioningReceiptOps,
        },
        fleet_registry::FleetRegistryOps,
    },
};

const CONFIG: &str = r#"
[app]
name = "service_binding_test"

[roles.root]
kind = "root"
package = "root"

[roles.database]
kind = "canister"
package = "database"

[roles.api]
kind = "canister"
package = "api"

[component_specs.database]
component_role = "database"
maximum_instances = 8

[component_specs.api]
component_role = "api"
maximum_instances = 8

[component_groups.database.components.database]
component_spec = "database"
service = "database"

[component_groups.api.components.api]
component_spec = "api"
service = "api"

[component_group_deployments.authoritative]
component_group = "database"
service_purpose = "authority"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[component_group_deployments.replicas]
component_group = "database"
service_purpose = "replica"
initial_placements = 1
maximum_placements = 3
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[component_group_deployments.api]
component_group = "api"
service_purpose = "pool_member"
initial_placements = 2
maximum_placements = 4
placement.maximum_per_root = 2
placement.minimum_distinct_roots = 2

[services.fleet.targets.database]
role = "database"
component_spec = "database"
mode = "authority_replica"
authority_deployment = "authoritative"
authority_member = ["database"]
placement.maximum_members_per_root = 1
placement.minimum_distinct_roots = 1

[services.fleet.targets.api]
role = "api"
component_spec = "api"
mode = "active_pool"
placement.maximum_members_per_root = 2
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
                    fleet_id: FleetId::from_generated_bytes([1; 32]),
                },
                app: AppId::from("service_binding_test"),
            },
            coordinator_subnet: subnet(2),
            coordinator: principal(3),
        },
        epoch: 1,
    }
}

fn release_set() -> FleetSubnetRootReleaseSet {
    FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([4; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([5; 32]),
    }
}

fn limits() -> FleetSubnetRootLimits {
    FleetSubnetRootLimits {
        maximum_component_instances: 8,
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
        maximum_group_placements: 4,
    }
}

fn root_entry(
    topology: &ComponentTopology,
    subnet_byte: u8,
    root_byte: u8,
) -> FleetSubnetRootEntry {
    let component_admissions = topology
        .component_specs
        .iter()
        .map(|spec| ComponentSpecAdmission {
            component_spec: spec.component_spec.clone(),
            spec_hash: spec.spec_hash,
            maximum_root_instances: 4,
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
        limits: limits(),
        status: FleetSubnetRootStatus::Joining,
    }
}

fn active_registry(config: &ConfigModel) -> FleetRegistry {
    let topology = config
        .compile_component_topology()
        .expect("Component Topology");
    let authority = authority();
    let mut registry = FleetRegistryOps::compile_genesis(
        &authority.binding.fleet.app,
        authority.clone(),
        &topology,
    )
    .expect("genesis Registry");
    for root in [root_entry(&topology, 6, 7), root_entry(&topology, 8, 9)] {
        registry = FleetRegistryOps::compile_joining(&authority, &topology, &registry, root)
            .expect("join root");
    }
    FleetRegistryOps::compile_active(&authority, &topology, &registry).expect("activate roots")
}

fn root_binding(registry: &FleetRegistry, root: &FleetSubnetRootEntry) -> FleetSubnetRootBinding {
    FleetSubnetRootBinding {
        authority: registry.authority.clone(),
        placement_subnet: root.placement_subnet,
        fleet_subnet_root: root.fleet_subnet_root,
        component_admissions: root.component_admissions.clone(),
        component_topology_digest: root.component_topology_digest,
        limits: root.limits.clone(),
    }
}

fn selected_root(deployment: &str, ordinal: u32) -> usize {
    match deployment {
        "authoritative" => 0,
        "replicas" => 1,
        "api" => usize::try_from(ordinal).expect("bounded API ordinal"),
        _ => panic!("unexpected deployment"),
    }
}

fn plan(config: &ConfigModel, registry: &FleetRegistry) -> FleetComponentProvisioningPlan {
    let topology = config
        .compile_component_group_deployment_topology()
        .expect("deployment topology");
    let mut placements_by_root = vec![Vec::new(); registry.fleet_subnet_roots.len()];
    for deployment in &topology.component_group_deployments {
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
        for ordinal in 0..deployment.initial_placements {
            placements_by_root[selected_root(deployment.deployment.as_str(), ordinal)].push(
                ComponentGroupPlacementPlan {
                    group_placement: ComponentGroupPlacementId {
                        deployment: deployment.deployment.clone(),
                        ordinal,
                    },
                    component_group: deployment.component_group.clone(),
                    entries: entries.clone(),
                },
            );
        }
    }
    let batches = registry
        .fleet_subnet_roots
        .iter()
        .zip(placements_by_root)
        .map(|(root, mut placements)| {
            placements.sort_by(|left, right| left.group_placement.cmp(&right.group_placement));
            FleetSubnetRootProvisioningBatch {
                root: root_binding(registry, root),
                active_release_set: root.active_release_set,
                placements,
            }
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
            &config
                .compile_component_topology()
                .expect("Component Topology"),
            registry,
        )
        .expect("Registry version"),
        configuration_digest: config
            .compile_component_deployment_configuration_digest()
            .expect("configuration digest"),
        operation: FleetComponentProvisioningOperation::FreshInstall,
        directory_confirmation_roots,
        batches,
    }
}

fn receipts(
    config: &ConfigModel,
    registry: &FleetRegistry,
    plan: &FleetComponentProvisioningPlan,
    operation_id: [u8; 32],
) -> Vec<RootComponentProvisioningStatusResponse> {
    let plan_hash = ComponentProvisioningPlanOps::hash(config, registry, plan).expect("plan hash");
    provisioned_receipts(config, plan, operation_id, plan_hash, 20)
}

fn provisioned_receipts(
    config: &ConfigModel,
    plan: &FleetComponentProvisioningPlan,
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    first_component: u8,
) -> Vec<RootComponentProvisioningStatusResponse> {
    let topology = config
        .compile_component_topology()
        .expect("Component Topology");
    let mut next_component = first_component;
    plan.batches
        .iter()
        .enumerate()
        .map(|(batch_index, batch)| {
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
                                let identity_byte = next_component;
                                next_component =
                                    next_component.checked_add(1).expect("test identity");
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
            let placement_count = u32::try_from(batch.placements.len()).expect("placement count");
            let component_count = batch
                .placements
                .iter()
                .map(|placement| u32::try_from(placement.entries.len()).expect("member count"))
                .sum();
            let accepted_at_ns = 100 + u64::try_from(batch_index).expect("batch index");
            let provisioned_at_ns = accepted_at_ns + 1;
            let receipt_content_hash =
                RootComponentProvisioningReceiptOps::provisioned_content_hash(
                    RootComponentProvisioningProvisionedReceiptAuthority {
                        operation_id,
                        plan_hash,
                        fleet_registry: &plan.fleet_registry,
                        configuration_digest: plan.configuration_digest,
                        root: &batch.root,
                        result: &result,
                        accepted_at_ns,
                        provisioned_at_ns,
                    },
                )
                .expect("receipt hash");
            RootComponentProvisioningStatusResponse {
                operation_id,
                plan_hash,
                fleet_registry: plan.fleet_registry.clone(),
                configuration_digest: plan.configuration_digest,
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
        })
        .collect()
}

fn scale_out_plan(
    config: &ConfigModel,
    registry: &FleetRegistry,
    fresh_plan: &FleetComponentProvisioningPlan,
) -> (FleetComponentProvisioningPlan, [u8; 32]) {
    let deployment_topology = config
        .compile_component_group_deployment_topology()
        .expect("deployment topology");
    let deployment = deployment_topology
        .get(&"api".parse().expect("deployment ID"))
        .expect("API deployment");
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
    let selected_root = &registry.fleet_subnet_roots[0];
    let plan = FleetComponentProvisioningPlan {
        fleet: registry.authority.binding.fleet.clone(),
        fleet_registry: FleetRegistryOps::version(
            &registry.authority,
            &config
                .compile_component_topology()
                .expect("Component Topology"),
            registry,
        )
        .expect("published Registry version"),
        configuration_digest: config
            .compile_component_deployment_configuration_digest()
            .expect("configuration digest"),
        operation: FleetComponentProvisioningOperation::ScaleOut {
            deployment: deployment.deployment.clone(),
            previous_placements: 2,
            requested_placements: 3,
        },
        directory_confirmation_roots: registry
            .fleet_subnet_roots
            .iter()
            .map(|root| root.fleet_subnet_root)
            .collect(),
        batches: vec![FleetSubnetRootProvisioningBatch {
            root: root_binding(registry, selected_root),
            active_release_set: selected_root.active_release_set,
            placements: vec![ComponentGroupPlacementPlan {
                group_placement: ComponentGroupPlacementId {
                    deployment: deployment.deployment.clone(),
                    ordinal: 2,
                },
                component_group: deployment.component_group.clone(),
                entries,
            }],
        }],
    };
    let mut committed_placements = fresh_plan
        .batches
        .iter()
        .flat_map(|batch| {
            batch
                .placements
                .iter()
                .map(|placement| ComponentProvisioningPlacementAuthority {
                    placement: placement.group_placement.clone(),
                    fleet_subnet_root: batch.root.fleet_subnet_root,
                })
        })
        .collect::<Vec<_>>();
    committed_placements.sort_unstable_by(|left, right| left.placement.cmp(&right.placement));
    let mut eligible_roots = fresh_plan
        .batches
        .iter()
        .map(|batch| batch.root.fleet_subnet_root)
        .collect::<Vec<_>>();
    eligible_roots.sort_unstable();
    let plan_hash = ComponentProvisioningPlanOps::hash_scale_out_compiled(
        &config
            .compile_component_deployment_configuration()
            .expect("deployment configuration"),
        registry,
        &plan,
        ComponentProvisioningScaleOutAuthority {
            committed_placements: &committed_placements,
            eligible_roots: &eligible_roots,
            next_placement_ordinal: 2,
        },
    )
    .expect("scale-out plan hash");
    (plan, plan_hash)
}

fn rehash(
    receipt: &mut RootComponentProvisioningStatusResponse,
    batch: &FleetSubnetRootProvisioningBatch,
) {
    receipt.receipt_content_hash = RootComponentProvisioningReceiptOps::provisioned_content_hash(
        RootComponentProvisioningProvisionedReceiptAuthority {
            operation_id: receipt.operation_id,
            plan_hash: receipt.plan_hash,
            fleet_registry: &receipt.fleet_registry,
            configuration_digest: receipt.configuration_digest,
            root: &batch.root,
            result: receipt.result.as_ref().expect("Provisioned result"),
            accepted_at_ns: receipt.accepted_at_ns,
            provisioned_at_ns: receipt.provisioned_at_ns.expect("completion time"),
        },
    )
    .expect("receipt hash");
}

fn fixture() -> (
    ConfigModel,
    FleetRegistry,
    FleetComponentProvisioningPlan,
    Vec<RootComponentProvisioningStatusResponse>,
) {
    let config = parse_config_model(CONFIG).expect("configuration");
    let registry = active_registry(&config);
    let plan = plan(&config, &registry);
    let receipts = receipts(&config, &registry, &plan, [10; 32]);
    (config, registry, plan, receipts)
}

#[test]
fn compiles_complete_mode_compatible_initial_services_in_canonical_order() {
    let (config, registry, plan, receipts) = fixture();
    assert_eq!(
        receipts[0].receipt_content_hash,
        [
            166, 136, 10, 151, 230, 206, 132, 114, 248, 141, 176, 138, 147, 210, 38, 93, 139, 163,
            18, 227, 204, 247, 154, 59, 41, 114, 251, 12, 97, 153, 41, 5,
        ]
    );
    let services = compile_initial(&config, &registry, &plan, [10; 32], &receipts)
        .expect("complete Fleet services");

    assert_eq!(
        services
            .iter()
            .map(|service| service.service.as_str())
            .collect::<Vec<_>>(),
        vec!["api", "database"]
    );
    assert_eq!(services[0].mode, FleetServiceMode::ActivePool);
    assert_eq!(services[0].members.len(), 2);
    assert!(
        services[0]
            .members
            .iter()
            .all(|member| member.member_purpose == FleetServiceMemberPurpose::PoolMember)
    );
    assert_eq!(services[1].mode, FleetServiceMode::AuthorityReplica);
    assert_eq!(
        services[1]
            .members
            .iter()
            .map(|member| member.member_purpose)
            .collect::<Vec<_>>(),
        vec![
            FleetServiceMemberPurpose::Authority,
            FleetServiceMemberPurpose::Replica
        ]
    );
    assert_eq!(
        candid::decode_one::<Vec<FleetServiceBinding>>(
            &candid::encode_one(&services).expect("encode service bindings")
        )
        .expect("decode service bindings"),
        services
    );
}

#[test]
fn compiles_one_complete_atomic_pool_member_scale_out() {
    let (config, active, fresh_plan, fresh_receipts) = fixture();
    let configuration = config
        .compile_component_deployment_configuration()
        .expect("deployment configuration");
    let initial_services =
        compile_initial(&config, &active, &fresh_plan, [10; 32], &fresh_receipts)
            .expect("initial services");
    let published = FleetRegistryOps::compile_initial_services(
        &active.authority,
        &configuration.component_topology,
        &active,
        initial_services,
    )
    .expect("publish initial services");
    let (scale_out, plan_hash) = scale_out_plan(&config, &published, &fresh_plan);
    let receipts = provisioned_receipts(&config, &scale_out, [40; 32], plan_hash, 50);

    let services = FleetServiceBindingOps::compile_scale_out_compiled(
        &configuration,
        &published,
        &scale_out,
        [40; 32],
        plan_hash,
        &receipts,
    )
    .expect("compile complete post-scale-out services");

    assert_eq!(services[0].service.as_str(), "api");
    assert_eq!(services[0].members.len(), 3);
    assert_eq!(services[1], published.services[1]);
    let appended = FleetRegistryOps::compile_service_additions(
        &published.authority,
        &configuration.component_topology,
        &published,
        services,
    )
    .expect("append all new PoolMembers atomically");
    assert_eq!(appended.revision, published.revision + 1);
}

#[test]
fn scale_out_rejects_existing_identity_reuse_and_authority_addition() {
    let (config, active, fresh_plan, fresh_receipts) = fixture();
    let configuration = config
        .compile_component_deployment_configuration()
        .expect("deployment configuration");
    let initial_services =
        compile_initial(&config, &active, &fresh_plan, [10; 32], &fresh_receipts)
            .expect("initial services");
    let published = FleetRegistryOps::compile_initial_services(
        &active.authority,
        &configuration.component_topology,
        &active,
        initial_services,
    )
    .expect("publish initial services");
    let (scale_out, plan_hash) = scale_out_plan(&config, &published, &fresh_plan);
    let mut receipts = provisioned_receipts(&config, &scale_out, [40; 32], plan_hash, 50);
    receipts[0].result.as_mut().expect("result").placements[0].members[0]
        .binding
        .component = published.services[0].members[0].component;
    rehash(&mut receipts[0], &scale_out.batches[0]);
    crate::assert_err_variant!(
        compile_scale_out_compiled_configuration(
            &configuration,
            &published,
            &scale_out,
            [40; 32],
            plan_hash,
            &receipts,
        ),
        Err(FleetServiceBindingOpsError::DuplicateComponentIdentity { .. })
    );

    let mut authority_plan = scale_out;
    authority_plan.batches[0].placements[0].entries[0].purpose =
        ComponentDeploymentPurpose::FleetServiceMember {
            service: "api".parse().expect("service ID"),
            member_purpose: FleetServiceMemberPurpose::Authority,
        };
    let mut authority_receipts =
        provisioned_receipts(&config, &authority_plan, [41; 32], [42; 32], 60);
    rehash(&mut authority_receipts[0], &authority_plan.batches[0]);
    crate::assert_err_variant!(
        compile_scale_out_compiled_configuration(
            &configuration,
            &published,
            &authority_plan,
            [41; 32],
            [42; 32],
            &authority_receipts,
        ),
        Err(FleetServiceBindingOpsError::InvalidScaleOutMemberPurpose)
    );
}

#[test]
fn rejects_missing_reordered_or_wrong_operation_root_receipts() {
    let (config, registry, plan, receipts) = fixture();
    crate::assert_err_variant!(
        compile_initial(&config, &registry, &plan, [10; 32], &receipts[..1]),
        Err(FleetServiceBindingOpsError::RootReceiptCountMismatch { .. })
    );

    let mut reordered = receipts.clone();
    reordered.reverse();
    crate::assert_err_variant!(
        compile_initial(&config, &registry, &plan, [10; 32], &reordered),
        Err(FleetServiceBindingOpsError::RootReceiptIdentityMismatch)
    );

    crate::assert_err_variant!(
        compile_initial(&config, &registry, &plan, [11; 32], &receipts),
        Err(FleetServiceBindingOpsError::RootReceiptIdentityMismatch)
    );
}

#[test]
fn rejects_corrupt_hash_and_validly_rehashed_result_substitution() {
    let (config, registry, plan, receipts) = fixture();
    let mut corrupt_hash = receipts.clone();
    corrupt_hash[0].receipt_content_hash = [99; 32];
    crate::assert_err_variant!(
        compile_initial(&config, &registry, &plan, [10; 32], &corrupt_hash),
        Err(FleetServiceBindingOpsError::RootReceiptInvalidHash)
    );

    let mut substituted = receipts;
    substituted[0].result.as_mut().expect("result").placements[0].members[0]
        .limits
        .maximum_descendants += 1;
    rehash(&mut substituted[0], &plan.batches[0]);
    crate::assert_err_variant!(
        compile_initial(&config, &registry, &plan, [10; 32], &substituted),
        Err(FleetServiceBindingOpsError::RootReceiptResultMismatch)
    );
}

#[test]
fn rejects_cross_root_identity_or_principal_reuse_after_exact_receipt_rehash() {
    let (config, registry, plan, receipts) = fixture();
    let mut reused_identity = receipts.clone();
    let reused = receipts[0].result.as_ref().expect("result").placements[0].members[0]
        .binding
        .component;
    reused_identity[1]
        .result
        .as_mut()
        .expect("result")
        .placements[0]
        .members[0]
        .binding
        .component = reused;
    rehash(&mut reused_identity[1], &plan.batches[1]);

    crate::assert_err_variant!(
        compile_initial(&config, &registry, &plan, [10; 32], &reused_identity),
        Err(FleetServiceBindingOpsError::DuplicateComponentIdentity { .. })
    );

    let mut reused_principal = receipts;
    let reused = reused_principal[0]
        .result
        .as_ref()
        .expect("result")
        .placements[0]
        .members[0]
        .binding
        .canister_id;
    reused_principal[1]
        .result
        .as_mut()
        .expect("result")
        .placements[0]
        .members[0]
        .binding
        .canister_id = reused;
    rehash(&mut reused_principal[1], &plan.batches[1]);
    crate::assert_err_variant!(
        compile_initial(&config, &registry, &plan, [10; 32], &reused_principal),
        Err(FleetServiceBindingOpsError::DuplicateComponentPrincipal { .. })
    );
}

#[test]
fn rejects_nonterminal_phase_counts_and_time_evidence() {
    let (config, registry, plan, receipts) = fixture();
    let mut wrong_phase = receipts.clone();
    wrong_phase[0].phase = RootComponentProvisioningPhase::Accepted;
    crate::assert_err_variant!(
        compile_initial(&config, &registry, &plan, [10; 32], &wrong_phase),
        Err(FleetServiceBindingOpsError::RootReceiptStateMismatch)
    );

    let mut wrong_count = receipts.clone();
    wrong_count[0].registry_committed_component_count -= 1;
    crate::assert_err_variant!(
        compile_initial(&config, &registry, &plan, [10; 32], &wrong_count),
        Err(FleetServiceBindingOpsError::RootReceiptCountsMismatch)
    );

    let mut wrong_time = receipts;
    wrong_time[0].provisioned_at_ns = Some(wrong_time[0].accepted_at_ns - 1);
    crate::assert_err_variant!(
        compile_initial(&config, &registry, &plan, [10; 32], &wrong_time),
        Err(FleetServiceBindingOpsError::RootReceiptTimeMismatch)
    );
}
