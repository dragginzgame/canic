//! Focused proof for typed current Fleet protocol compilation.

use super::*;
use crate::fleet_ensure::model::{
    DesiredCanister, DesiredCanisterKind, DesiredComponentGroupPlacement, DesiredFleetBootstrap,
    DesiredFleetBootstrapRoot, DesiredFleetProtocol, FLEET_ENSURE_SCHEMA_VERSION,
};
use canic_core::{
    bootstrap::parse_config_model,
    cdk::types::Cycles,
    control_plane_support::ops::fleet_registry::FleetRegistryOps,
    dto::fleet_registry::{FleetRegistry, FleetSubnetRootEntry, FleetSubnetRootStatus},
    dto::fleet_subnet_root::FleetSubnetRootAuthority,
    ids::{
        AppId, CanonicalNetworkId, ComponentSpecAdmission, CyclesFundingBudget, FleetBinding,
        FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority,
        FleetSubnetCanisterPoolConfig, FleetSubnetRootLimits, FleetSubnetRootReleaseSet,
        FleetSubnetWasmStoreAuthority, ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest,
        SubnetId,
    },
    shared_support::fleet_admission_policy::{
        bind_initial_fleet_admission_policy, compile_fleet_admission_policy_template,
    },
};
use flate2::{Compression, GzBuilder};
use std::{fs, io::Write};

const CONFIG: &str = r#"
[app]
name = "ensure_protocol_test"

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

[component_group_deployments.cells]
component_group = "cell"
initial_placements = 2
maximum_placements = 4
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 2
"#;

#[test]
fn typed_placements_compile_to_exact_active_root_batches() {
    let config = parse_config_model(CONFIG).expect("valid Component deployment config");
    let configuration = config
        .compile_component_deployment_configuration()
        .expect("compiled deployment configuration");
    let registry = active_registry(&config);
    let desired = desired(vec![
        placement("cells", 1, "root-two"),
        placement("cells", 0, "root-one"),
    ]);
    let state = state();

    let placements = resolve_placements(&desired, &state, &registry).expect("resolve Roots");
    let compiled =
        compile_current_component_provisioning(&configuration, &registry, [42; 32], &placements)
            .expect("compile exact typed plan");
    let plan = &compiled.request.plan;

    assert_eq!(plan.batches.len(), 2);
    assert!(
        plan.batches
            .is_sorted_by_key(|batch| batch.root.fleet_subnet_root)
    );
    let assignments = plan
        .batches
        .iter()
        .map(|batch| {
            (
                batch.root.fleet_subnet_root,
                batch.placements[0].group_placement.ordinal,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(assignments, vec![(principal(10), 1), (principal(20), 0)]);
    assert_eq!(
        plan.operation,
        FleetComponentProvisioningOperation::FreshInstall
    );
}

#[test]
fn duplicate_or_incomplete_placement_authority_fails_closed() {
    let config = parse_config_model(CONFIG).expect("valid Component deployment config");
    let configuration = config
        .compile_component_deployment_configuration()
        .expect("compiled deployment configuration");
    let registry = active_registry(&config);
    let state = state();

    let duplicate = desired(vec![
        placement("cells", 0, "root-one"),
        placement("cells", 0, "root-two"),
    ]);
    assert!(matches!(
        resolve_placements(&duplicate, &state, &registry).and_then(|placements| {
            compile_current_component_provisioning(&configuration, &registry, [42; 32], &placements)
        }),
        Err(CurrentProtocolError::InvalidPlacement(_))
    ));

    let incomplete = desired(vec![placement("cells", 0, "root-one")]);
    assert!(matches!(
        resolve_placements(&incomplete, &state, &registry).and_then(|placements| {
            compile_current_component_provisioning(&configuration, &registry, [42; 32], &placements)
        }),
        Err(CurrentProtocolError::InvalidPlacement(_))
    ));
}

#[test]
fn live_root_authority_compiles_one_deterministic_registry_sequence() {
    let config = parse_config_model(CONFIG).expect("valid Component deployment config");
    let topology = config
        .compile_component_topology()
        .expect("Component topology");
    let active = active_registry(&config);
    let genesis = FleetRegistryOps::compile_genesis(
        &active.authority.binding.fleet.app,
        active.authority.clone(),
        &topology,
        active.admission.clone(),
    )
    .expect("genesis Registry");
    let desired = desired(vec![
        placement("cells", 0, "root-one"),
        placement("cells", 1, "root-two"),
    ]);
    let authorities = root_authorities(&active);

    let sequence =
        compile_current_registry_sequence(&desired, &state(), &topology, &genesis, &authorities)
            .expect("compile canonical Registry sequence");

    assert_eq!(sequence.current_stage, CurrentRegistryStage::Genesis);
    assert_eq!(sequence.joins.len(), 2);
    assert_eq!(
        sequence
            .joins
            .iter()
            .map(|join| join.request.entry.fleet_subnet_root)
            .collect::<Vec<_>>(),
        vec![principal(20), principal(10)]
    );
    assert_eq!(sequence.joins[0].request.expected_registry.revision, 1);
    assert_eq!(sequence.joins[1].request.expected_registry.revision, 2);
    assert_eq!(sequence.activation_request.expected_registry.revision, 3);
    assert_eq!(sequence.active_registry, active);

    let one_join = compile_current_registry_sequence(
        &desired,
        &state(),
        &topology,
        &sequence.joins[0].resulting_registry,
        &authorities,
    )
    .expect("recognize exact Joining prefix");
    assert_eq!(one_join.current_stage, CurrentRegistryStage::Joining(1));

    let terminal =
        compile_current_registry_sequence(&desired, &state(), &topology, &active, &authorities)
            .expect("recognize exact Active Registry");
    assert_eq!(terminal.current_stage, CurrentRegistryStage::Active);
}

#[test]
fn registry_sequence_rejects_store_and_registry_authority_drift() {
    let config = parse_config_model(CONFIG).expect("valid Component deployment config");
    let topology = config
        .compile_component_topology()
        .expect("Component topology");
    let active = active_registry(&config);
    let genesis = FleetRegistryOps::compile_genesis(
        &active.authority.binding.fleet.app,
        active.authority.clone(),
        &topology,
        active.admission.clone(),
    )
    .expect("genesis Registry");
    let mut mismatched_desired = desired(vec![
        placement("cells", 0, "root-one"),
        placement("cells", 1, "root-two"),
    ]);
    let authorities = root_authorities(&active);

    mismatched_desired
        .canisters
        .iter_mut()
        .find(|canister| canister.name == "store-one")
        .expect("Store")
        .principal = Some(principal(99).to_string());
    assert!(matches!(
        compile_current_registry_sequence(
            &mismatched_desired,
            &state(),
            &topology,
            &genesis,
            &authorities,
        ),
        Err(CurrentProtocolError::RegistrySequenceConflict(_))
    ));

    let desired = desired(vec![
        placement("cells", 0, "root-one"),
        placement("cells", 1, "root-two"),
    ]);
    let mut drifted = genesis;
    drifted.revision = 9;
    assert!(matches!(
        compile_current_registry_sequence(&desired, &state(), &topology, &drifted, &authorities,),
        Err(CurrentProtocolError::RegistrySequenceConflict(_))
    ));
}

#[test]
fn provisioned_registry_requires_its_exact_component_operation_receipt() {
    let config = parse_config_model(CONFIG).expect("valid Component deployment config");
    let topology = config
        .compile_component_topology()
        .expect("Component topology");
    let configuration = config
        .compile_component_deployment_configuration()
        .expect("Component deployment configuration");
    let active = active_registry(&config);
    let desired = desired(vec![
        placement("cells", 0, "root-one"),
        placement("cells", 1, "root-two"),
    ]);
    let state = state();
    let authorities = root_authorities(&active);
    let placements = resolve_placements(&desired, &state, &active).expect("resolve placements");
    let compiled =
        compile_current_component_provisioning(&configuration, &active, [42; 32], &placements)
            .expect("compile Component operation");
    let mut published = active;
    published.revision = published.revision.checked_add(1).expect("next revision");
    let published_version = registry_version(&topology, &published).expect("published version");
    let status =
        canic_core::dto::component_provisioning::FleetComponentProvisioningStatusResponse {
            operation_id: compiled.request.operation_id,
            plan_hash: compiled.plan_hash,
            fleet_registry: compiled.request.plan.fleet_registry.clone(),
            configuration_digest: compiled.request.plan.configuration_digest,
            operation: compiled.request.plan.operation.clone(),
            phase: FleetComponentProvisioningPhase::RuntimesActivated,
            directory_confirmation_root_count: 2,
            root_batch_count: 2,
            accepted_root_count: 2,
            acceptance_in_flight_root: None,
            provisioned_root_count: 2,
            current_root: None,
            provisioning_in_flight_root: None,
            directory_confirmed_root_count: 2,
            current_synchronization: None,
            current_publication: None,
            publication_in_flight_root: None,
            runtime_activated_root_count: 2,
            current_activation: None,
            activation_in_flight_root: None,
            pending_root_failure: None,
            group_placement_count: 2,
            component_count: 2,
            planned_at_ns: 1,
            roots_accepted_at_ns: Some(2),
            components_provisioned_at_ns: Some(3),
            published_fleet_registry: Some(published_version),
            service_topology_published_at_ns: Some(4),
            directories_confirmed_at_ns: Some(5),
            runtimes_activated_at_ns: Some(6),
        };

    assert_retry_timestamp_is_not_durable_progress(&status);

    let sequence = compile_current_registry_sequence_with_status(
        &desired,
        &state,
        &topology,
        &published,
        &authorities,
        Some(&status),
    )
    .expect("recognize exact published successor");
    assert_eq!(sequence.current_stage, CurrentRegistryStage::Provisioned);
    require_component_status_matches(&status, &compiled.request, compiled.plan_hash)
        .expect("bind exact compiled operation");

    let mut drifted_registry = published;
    drifted_registry.revision = drifted_registry
        .revision
        .checked_add(1)
        .expect("drifted revision");
    assert!(matches!(
        compile_current_registry_sequence_with_status(
            &desired,
            &state,
            &topology,
            &drifted_registry,
            &authorities,
            Some(&status),
        ),
        Err(CurrentProtocolError::RegistrySequenceConflict(_))
    ));

    let mut drifted_status = status;
    drifted_status.plan_hash[0] ^= 1;
    assert!(matches!(
        require_component_status_matches(&drifted_status, &compiled.request, compiled.plan_hash),
        Err(CurrentProtocolError::RegistrySequenceConflict(_))
    ));
}

fn assert_retry_timestamp_is_not_durable_progress(
    status: &canic_core::dto::component_provisioning::FleetComponentProvisioningStatusResponse,
) {
    let mut first_failure = status.clone();
    first_failure.phase = FleetComponentProvisioningPhase::ActivatingRuntimes;
    first_failure.pending_root_failure = Some(
        canic_core::dto::component_provisioning::FleetComponentProvisioningRootFailure {
            fleet_subnet_root: principal(10),
            stage: canic_core::dto::component_provisioning::FleetComponentProvisioningRetryStage::RuntimeActivation,
            diagnostic_code: canic_core::diagnostics::codes::STATE_CONFLICT
                .raw_code()
                .raw(),
            failed_at_ns: 10,
        },
    );
    let mut repeated_failure = first_failure.clone();
    repeated_failure
        .pending_root_failure
        .as_mut()
        .expect("failure")
        .failed_at_ns = 20;
    assert_eq!(
        component_provisioning_observation(false, &first_failure)
            .expect("first failure identity")
            .progress_identity,
        component_provisioning_observation(false, &repeated_failure)
            .expect("repeated failure identity")
            .progress_identity,
    );
}

#[test]
fn store_authority_without_operation_receipt_does_not_complete_adoption() {
    let config = parse_config_model(CONFIG).expect("valid Component deployment config");
    let registry = active_registry(&config);
    let authority = root_authorities(&registry)
        .into_iter()
        .next()
        .expect("Root authority")
        .wasm_store_authority;
    let request = FleetSubnetWasmStoreAdoptionRequest {
        operation_id: [44; 32],
        authority: authority.clone(),
    };

    assert!(!store_adoption_applied(&request, None));
    let exact = canic_core::dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionResponse {
        operation_id: request.operation_id,
        authority: authority.clone(),
        controllers: expected_store_controllers(&authority),
        adopted_at_ns: 1,
    };
    assert!(store_adoption_applied(&request, Some(&exact)));

    let mut conflicting = exact;
    conflicting.operation_id[0] ^= 1;
    assert!(!store_adoption_applied(&request, Some(&conflicting)));
}

#[test]
fn current_desired_state_rejects_component_demand_above_pool_target() {
    let root = crate::test_support::temp_dir("current-protocol-pool-capacity");
    fs::create_dir_all(&root).expect("create test root");
    fs::write(root.join("canic.toml"), CONFIG).expect("write App config");
    let config = parse_config_model(CONFIG).expect("valid Component deployment config");
    let registry = active_registry(&config);
    let mut desired = desired(vec![
        placement("cells", 0, "root-one"),
        placement("cells", 1, "root-two"),
    ]);
    let authorities = root_authorities(&registry);
    let mut roots = authorities
        .iter()
        .map(|authority| DesiredFleetBootstrapRoot {
            canister_pool_imports: Vec::new(),
            component_admissions: authority.binding.component_admissions.clone(),
            component_topology_digest: authority.binding.component_topology_digest,
            funding: authority.binding.funding.clone(),
            limits: authority.binding.limits.clone(),
            placement_subnet: authority.binding.placement_subnet,
            root: if authority.binding.fleet_subnet_root == principal(20) {
                "root-one".to_string()
            } else {
                "root-two".to_string()
            },
            store: if authority.binding.fleet_subnet_root == principal(20) {
                "store-one".to_string()
            } else {
                "store-two".to_string()
            },
        })
        .collect::<Vec<_>>();
    roots[0].limits.canister_pool.canister_cycles = Cycles::new(4_999_999_999_999);
    desired.bootstrap = Some(DesiredFleetBootstrap {
        admission: compile_fleet_admission_policy_template(vec![principal(1)], Vec::new())
            .expect("Fleet admission template"),
        app: registry.authority.binding.fleet.app.clone(),
        canonical_network_id: registry.authority.binding.fleet.fleet.canonical_network_id,
        component_deployment_configuration: config
            .compile_component_deployment_configuration()
            .expect("Component deployment configuration"),
        coordinator: "coordinator".to_string(),
        coordinator_subnet: registry.authority.binding.coordinator_subnet,
        fleet_id: registry.authority.binding.fleet.fleet.fleet_id,
        release_build_id: authorities[0].initial_release_set.release_build_id,
        root_funding: None,
        roots,
    });

    assert!(matches!(
        validate_component_pool_capacity(&root, &desired),
        Err(CurrentProtocolError::ComponentPoolCapacity(
            crate::component_topology::RootPoolCapacityError::Insufficient {
                pool_target_cycles: 4_999_999_999_999,
                required_cycles: 5_000_000_000_000,
                ..
            }
        ))
    ));
}

#[test]
fn store_sequence_binds_qualified_bytes_and_deterministic_replay_identities() {
    let root = crate::test_support::temp_dir("current-store-sequence");
    let release = crate::release_build::plan_release_build(&root).expect("plan release build");
    let release_build_id = release.record.release_build_id;
    let config = parse_config_model(CONFIG).expect("valid Component deployment config");
    let topology = config
        .compile_component_topology()
        .expect("Component topology");
    let artifact_root = root.join(".icp/local/canisters/alpha");
    fs::create_dir_all(&artifact_root).expect("create artifact root");
    let wasm_path = artifact_root.join("alpha.wasm");
    let wasm_gz_path = artifact_root.join("alpha.wasm.gz");
    let mut wasm = vec![0x00, 0x61, 0x73, 0x6d, 7];
    wasm.extend_from_slice(release_build_id.to_string().as_bytes());
    fs::write(&wasm_path, &wasm).expect("write Wasm");
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder.write_all(&wasm).expect("write gzip payload");
    fs::write(&wasm_gz_path, encoder.finish().expect("finish gzip")).expect("write gzip Wasm");
    let targets = vec![crate::release_set::ApplicationArtifactBuildTarget {
        role: CanisterRole::new("alpha"),
        package: "alpha".to_string(),
        wasm_relative_path: ".icp/local/canisters/alpha/alpha.wasm".to_string(),
        wasm_gz_relative_path: ".icp/local/canisters/alpha/alpha.wasm.gz".to_string(),
    }];
    let outputs = vec![crate::release_set::ApplicationArtifactFileBuildOutput {
        role: CanisterRole::new("alpha"),
        package: "alpha".to_string(),
        release_build_id,
        wasm_path,
        wasm_gz_path,
        candid_sha256: [3; 32],
        protocol_profile_digest: canic_core::role_contract::ProtocolProfileDigest::from_bytes(
            [4; 32],
        ),
    }];
    let persisted = crate::release_set::compile_and_persist_application_artifact_union(
        &root,
        &topology,
        release_build_id,
        &targets,
        &outputs,
    )
    .expect("persist qualified union");
    let registry = active_registry(&config);
    let entry = registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| entry.fleet_subnet_root == principal(20))
        .expect("Root entry");
    let registry_authority = registry.authority.clone();
    let binding = FleetSubnetRootBinding {
        authority: registry_authority.clone(),
        placement_subnet: entry.placement_subnet,
        fleet_subnet_root: entry.fleet_subnet_root,
        component_admissions: entry.component_admissions.clone(),
        component_topology_digest: entry.component_topology_digest,
        limits: entry.limits.clone(),
        funding: entry.funding.clone(),
    };
    let manifest =
        FleetSubnetRootReleaseSetManifest::project(&topology, &binding, &persisted.union)
            .expect("project Root release set");
    let manifest_bytes =
        serde_json::to_vec(&manifest.root_store_manifest()).expect("canonical manifest");
    let authority = FleetSubnetRootAuthority {
        binding,
        initial_release_set: FleetSubnetRootReleaseSet {
            release_build_id,
            manifest_digest: ReleaseSetDigest::from_bytes(Sha256::digest(manifest_bytes).into()),
        },
        expected_module_hash: [31; 32],
        wasm_store_authority: FleetSubnetWasmStoreAuthority {
            authority: registry_authority,
            placement_subnet: entry.placement_subnet,
            fleet_subnet_root: entry.fleet_subnet_root,
            wasm_store: principal(21),
            installation_controller: principal(50),
            release_build_id,
            wasm_module_hash: [32; 32],
        },
    };
    let compiled = compile_current_store_sequence(&root, &topology, &authority, [42; 32])
        .expect("compile exact Store sequence");
    let repeated = compile_current_store_sequence(&root, &topology, &authority, [42; 32])
        .expect("repeat exact Store sequence");

    assert_eq!(compiled, repeated);
    assert!(matches!(
        compiled.actions.first(),
        Some(CurrentFleetProtocolAction::PrepareStoreChunkSet { .. })
    ));
    assert!(matches!(
        compiled.actions.last(),
        Some(CurrentFleetProtocolAction::BootstrapStore { .. })
    ));
    assert_eq!(compiled.expected_bootstrap.catalog.len(), 1);
    assert_ne!(compiled.bootstrap_request.operation_id, [0; 32]);

    fs::remove_dir_all(root).expect("remove test root");
}

fn desired(placements: Vec<DesiredComponentGroupPlacement>) -> DesiredFleet {
    DesiredFleet {
        bootstrap: None,
        canisters: vec![
            canister(
                "coordinator",
                DesiredCanisterKind::Coordinator,
                principal(30),
                None,
                subnet(1),
            ),
            canister(
                "root-one",
                DesiredCanisterKind::Root,
                principal(20),
                Some("coordinator"),
                subnet(6),
            ),
            canister(
                "store-one",
                DesiredCanisterKind::Store,
                principal(21),
                Some("root-one"),
                subnet(6),
            ),
            canister(
                "root-two",
                DesiredCanisterKind::Root,
                principal(10),
                Some("coordinator"),
                subnet(7),
            ),
            canister(
                "store-two",
                DesiredCanisterKind::Store,
                principal(11),
                Some("root-two"),
                subnet(7),
            ),
        ],
        cycles_ledger: principal(40).to_string(),
        environment: "local".to_string(),
        fleet: "protocol-test".to_string(),
        ledger_fee_cycles: "0".to_string(),
        management_creation_fee_cycles: "0".to_string(),
        material_cycle_threshold: "0".to_string(),
        maximum_observation_burn_cycles: "0".to_string(),
        maximum_stalled_observations: 2,
        maximum_update_burn_cycles: "0".to_string(),
        operator: principal(50).to_string(),
        protocol: Some(DesiredFleetProtocol {
            app_config: "canic.toml".to_string(),
            component_group_placements: placements,
            coordinator_candid: "coordinator.did".to_string(),
            root_candid: "root.did".to_string(),
            store_candid: "store.did".to_string(),
        }),
        protocol_steps: Vec::new(),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        treasury: principal(60).to_string(),
    }
}

fn canister(
    name: &str,
    kind: DesiredCanisterKind,
    canister_principal: Principal,
    parent: Option<&str>,
    placement_subnet: SubnetId,
) -> DesiredCanister {
    let mut controllers = vec![principal(50).to_string()];
    if kind == DesiredCanisterKind::Store {
        controllers.push(match parent {
            Some("root-one") => principal(20).to_string(),
            Some("root-two") => principal(10).to_string(),
            _ => panic!("Store test fixture requires one known Root parent"),
        });
        controllers.sort();
    }
    DesiredCanister {
        canic_init: None,
        controllers,
        drain: None,
        initial_cycles: "0".to_string(),
        init_arg: None,
        init_candid: None,
        kind,
        minimum_cycles: "0".to_string(),
        name: name.to_string(),
        parent: parent.map(str::to_string),
        presence: DesiredPresence::Present,
        principal: Some(canister_principal.to_string()),
        protocol_binding: None,
        replace: false,
        subnet: placement_subnet.to_string(),
        wasm: None,
    }
}

fn root_authorities(registry: &FleetRegistry) -> Vec<FleetSubnetRootAuthority> {
    registry
        .fleet_subnet_roots
        .iter()
        .map(|entry| FleetSubnetRootAuthority {
            binding: FleetSubnetRootBinding {
                authority: registry.authority.clone(),
                placement_subnet: entry.placement_subnet,
                fleet_subnet_root: entry.fleet_subnet_root,
                component_admissions: entry.component_admissions.clone(),
                component_topology_digest: entry.component_topology_digest,
                limits: entry.limits.clone(),
                funding: entry.funding.clone(),
            },
            initial_release_set: entry.active_release_set,
            expected_module_hash: [31; 32],
            wasm_store_authority: FleetSubnetWasmStoreAuthority {
                authority: registry.authority.clone(),
                placement_subnet: entry.placement_subnet,
                fleet_subnet_root: entry.fleet_subnet_root,
                wasm_store: if entry.fleet_subnet_root == principal(20) {
                    principal(21)
                } else {
                    principal(11)
                },
                installation_controller: principal(50),
                release_build_id: entry.active_release_set.release_build_id,
                wasm_module_hash: [32; 32],
            },
        })
        .collect()
}

fn placement(deployment: &str, ordinal: u32, root: &str) -> DesiredComponentGroupPlacement {
    DesiredComponentGroupPlacement {
        deployment: deployment.to_string(),
        ordinal,
        root: root.to_string(),
    }
}

fn state() -> FleetEnsureStateRecord {
    FleetEnsureStateRecord {
        active_registry: None,
        completed_reinstalls: BTreeMap::new(),
        fleet: "protocol-test".to_string(),
        pending_principals: BTreeMap::new(),
        principals: BTreeMap::new(),
        retained_cycles_by_principal: BTreeMap::new(),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        topology: BTreeMap::new(),
    }
}

fn active_registry(config: &canic_core::bootstrap::compiled::ConfigModel) -> FleetRegistry {
    let topology = config
        .compile_component_topology()
        .expect("Component topology");
    let fleet = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([7; 32]),
        },
        app: AppId::from("ensure_protocol_test"),
    };
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: fleet.clone(),
            coordinator_subnet: subnet(1),
            coordinator: principal(30),
        },
        epoch: 1,
    };
    let template = compile_fleet_admission_policy_template(vec![principal(1)], Vec::new())
        .expect("Fleet admission template");
    let admission = bind_initial_fleet_admission_policy(fleet.clone(), &template)
        .expect("Fleet admission policy");
    let mut registry =
        FleetRegistryOps::compile_genesis(&fleet.app, authority.clone(), &topology, admission)
            .expect("genesis Registry");
    let release_set = FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([8; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
    };
    for root in [principal(20), principal(10)] {
        registry = FleetRegistryOps::compile_joining(
            &authority,
            &topology,
            &registry,
            FleetSubnetRootEntry {
                placement_subnet: subnet(if root == principal(20) { 6 } else { 7 }),
                fleet_subnet_root: root,
                component_admissions: vec![admission_for(&topology)],
                component_topology_digest: topology
                    .project_for_admissions(&[admission_for(&topology)])
                    .expect("root topology")
                    .digest()
                    .expect("root topology digest"),
                active_release_set: release_set,
                limits: limits(),
                funding: crate::test_support::fleet_subnet_root_funding_authority(),
                status: FleetSubnetRootStatus::Joining,
            },
        )
        .expect("Joining root");
    }
    FleetRegistryOps::compile_active(&authority, &topology, &registry).expect("active Registry")
}

fn admission_for(
    topology: &canic_core::bootstrap::compiled::ComponentTopology,
) -> ComponentSpecAdmission {
    let component_spec = "alpha".parse().expect("Component Spec ID");
    ComponentSpecAdmission {
        spec_hash: topology
            .get(&component_spec)
            .expect("Component Spec")
            .spec_hash,
        component_spec,
        maximum_root_instances: 2,
    }
}

fn limits() -> FleetSubnetRootLimits {
    FleetSubnetRootLimits {
        maximum_component_instances: 4,
        maximum_registry_bytes: 16_777_216,
        maximum_wasm_store_bytes: 40_000_000,
        maximum_group_placements: 2,
        canister_pool: FleetSubnetCanisterPoolConfig {
            minimum_size: 1,
            maximum_size: 2,
            canister_cycles: Cycles::new(5_000_000_000_000),
        },
        cycles_funding: CyclesFundingBudget {
            window_secs: 3_600,
            maximum_cycles: Cycles::new(10_000_000_000_000),
        },
    }
}

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(principal(byte))
}
