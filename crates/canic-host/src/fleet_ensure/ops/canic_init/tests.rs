//! Focused wire-format evidence for generated infrastructure initialization.

use super::*;
use canic_core::{
    bootstrap::parse_config_model,
    cdk::types::Cycles,
    control_plane_support::config::ComponentDeploymentConfiguration,
    ids::{
        AppId, CanonicalNetworkId, ComponentSpecAdmission, CyclesFundingBudget,
        FleetAdmissionPolicyTemplate, FleetId, FleetSubnetCanisterPoolConfig,
        FleetSubnetRootLimits, FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce,
        ReleaseSetDigest, SubnetId,
    },
    shared_support::fleet_admission_policy::{
        bind_initial_fleet_admission_policy, compile_fleet_admission_policy_template,
    },
};

const CONFIG: &str = r#"
[app]
name = "init_wire_test"

[roles.root]
kind = "root"

[roles.app]
kind = "canister"
package = "app"

[component_specs.app]
component_role = "app"
maximum_instances = 1
initial_cycles = "5T"

[component_groups.app.components.app]
component_spec = "app"

[component_group_deployments.app]
component_group = "app"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#;

/// Exercise shared release evidence using the generated, finalized native estate fixture.
pub(in crate::fleet_ensure) fn qualify_release_input_reuse(root: &Path, desired: &DesiredFleet) {
    qualify_compiled_initializers(root, desired);
    let mut desired = desired.clone();
    let mut principals = desired
        .canisters
        .iter()
        .filter_map(|canister| {
            canister
                .principal
                .as_ref()
                .map(|principal| (canister.name.clone(), principal.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let bootstrap = desired.bootstrap.as_mut().unwrap();
    let template = bootstrap.roots[0].clone();
    for index in 1..4_u8 {
        let mut next = template.clone();
        next.root = format!("extra-root-{index}");
        next.store = format!("extra-store-{index}");
        principals.insert(
            next.root.clone(),
            Principal::from_slice(&[index, 80]).to_text(),
        );
        principals.insert(
            next.store.clone(),
            Principal::from_slice(&[index, 81]).to_text(),
        );
        bootstrap.roots.push(next);
    }
    let complete =
        load_persisted_current_release_set_manifest(root, bootstrap.release_build_id).unwrap();
    let fixtures = complete
        .manifest
        .verify_fixtures(
            root,
            &bootstrap
                .component_deployment_configuration
                .component_topology,
        )
        .unwrap();
    let directory = complete.path.parent().unwrap();
    let together = compile_root_authorities(root, &desired, &principals).unwrap();
    let separately = desired
        .bootstrap
        .as_ref()
        .unwrap()
        .roots
        .iter()
        .map(|input| {
            let mut one = desired.clone();
            one.bootstrap.as_mut().unwrap().roots = vec![input.clone()];
            compile_root_authorities(root, &one, &principals)
                .unwrap()
                .pop()
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        together, separately,
        "shared inputs preserve every Root authority"
    );

    for path in [
        complete.path.clone(),
        directory.join("infrastructure-artifact-manifest.json"),
        directory.join("application-artifact-union.json"),
        fixtures.path,
    ] {
        let original = std::fs::read(&path).unwrap();
        std::fs::write(&path, b"{}").unwrap();
        assert!(
            matches!(
                compile_root_authorities(root, &desired, &principals),
                Err(CanicInitError::Release(_))
            ),
            "a subsequent compilation must reject changed release evidence"
        );
        std::fs::write(&path, original).unwrap();
        assert_eq!(
            compile_root_authorities(root, &desired, &principals).unwrap(),
            together
        );
    }
}

fn qualify_compiled_initializers(root: &Path, desired: &DesiredFleet) {
    let principals = desired
        .canisters
        .iter()
        .filter_map(|canister| {
            canister
                .principal
                .as_ref()
                .map(|principal| (canister.name.clone(), principal.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let authorities = compile_root_authorities(root, desired, &principals).unwrap();
    let bootstrap = desired.bootstrap.as_ref().unwrap();
    let infrastructure =
        load_persisted_canic_infrastructure_artifact_manifest(root, bootstrap.release_build_id)
            .unwrap();
    for (init, role) in [
        (
            DesiredCanisterInit::Coordinator,
            CanicInfrastructureRole::FleetCoordinator,
        ),
        (
            DesiredCanisterInit::Root {
                root: bootstrap.roots[0].root.clone(),
            },
            CanicInfrastructureRole::FleetSubnetRoot,
        ),
        (
            DesiredCanisterInit::Store {
                root: bootstrap.roots[0].root.clone(),
            },
            CanicInfrastructureRole::WasmStore,
        ),
    ] {
        let artifact = infrastructure_entry(&infrastructure.manifest, role).unwrap();
        let bytes = compile_arguments(&CanicInitRequest {
            desired,
            init: &init,
            operation_id: "authority-input-reuse",
            principals: &principals,
            root,
            wasm: &artifact.wasm_relative_path,
            wasm_sha256: &artifact.wasm_sha256_hex,
        })
        .unwrap();
        match init {
            DesiredCanisterInit::Coordinator => {
                let args: FleetCoordinatorInitArgs = candid::decode_one(&bytes).unwrap();
                assert_eq!(args.authority, authorities[0].1.binding.authority);
            }
            DesiredCanisterInit::Root { .. } => {
                let args: FleetSubnetRootInitArgs = candid::decode_one(&bytes).unwrap();
                assert_eq!(args.authority, authorities[0].1);
            }
            DesiredCanisterInit::Store { .. } => {
                let args: FleetSubnetWasmStoreInitArgs = candid::decode_one(&bytes).unwrap();
                assert_eq!(args.authority, authorities[0].1.wasm_store_authority);
            }
        }
    }
}

#[test]
fn generated_coordinator_root_and_store_init_bytes_decode_to_exact_authority() {
    let fixture = fixture();
    let template: FleetAdmissionPolicyTemplate =
        compile_fleet_admission_policy_template(vec![principal(9)], Vec::new())
            .expect("admission template");
    let admission =
        bind_initial_fleet_admission_policy(fixture.registry.binding.fleet.clone(), &template)
            .expect("initial admission");

    let coordinator_bytes =
        encode_coordinator_arguments(&fixture.bootstrap, fixture.registry.clone(), admission)
            .expect("Coordinator bytes");
    let coordinator: FleetCoordinatorInitArgs =
        candid::decode_one(&coordinator_bytes).expect("decode Coordinator init");
    assert_eq!(coordinator.authority, fixture.registry);
    assert_eq!(coordinator.configured_app, fixture.bootstrap.app);
    assert_eq!(
        coordinator.component_deployment_configuration,
        fixture.bootstrap.component_deployment_configuration
    );

    let operation_id = "ab".repeat(32);
    let pool = principal(4);
    let root_bytes = encode_root_arguments(
        fixture.root_authority.clone(),
        &operation_id,
        "root-0",
        vec![fixture.root_authority.binding.fleet_subnet_root],
        vec![pool],
    )
    .expect("Root bytes");
    let root: FleetSubnetRootInitArgs = candid::decode_one(&root_bytes).expect("decode Root init");
    assert_eq!(root.authority, fixture.root_authority);
    assert_eq!(root.canister_pool_imports, vec![pool]);
    assert_eq!(root.install_id, install_id(&operation_id, "root", "root-0"));
    assert_eq!(
        root.wasm_store_activation.operation_id,
        install_id(&operation_id, "store", "root-0")
    );
    assert_eq!(
        root.wasm_store_activation.wasm_store,
        fixture.root_authority.wasm_store_authority.wasm_store
    );
    assert_eq!(
        root.wasm_store_activation.controllers,
        vec![fixture.root_authority.binding.fleet_subnet_root]
    );

    let store_authority = root.authority.wasm_store_authority.clone();
    let store_bytes = encode_store_arguments(store_authority.clone(), &operation_id, "root-0")
        .expect("Store bytes");
    let store: FleetSubnetWasmStoreInitArgs =
        candid::decode_one(&store_bytes).expect("decode Store init");
    assert_eq!(store.authority, store_authority);
    assert_eq!(
        store.install_id,
        install_id(&operation_id, "store", "root-0")
    );
    assert_ne!(root.install_id, store.install_id);
    assert_eq!(root.wasm_store_activation.operation_id, store.install_id);
}

struct InitFixture {
    bootstrap: DesiredFleetBootstrap,
    registry: FleetRegistryAuthority,
    root_authority: FleetSubnetRootAuthority,
}

fn fixture() -> InitFixture {
    let config = parse_config_model(CONFIG).expect("config");
    let deployment = ComponentDeploymentConfiguration::compile(&config).expect("deployment");
    let topology = &deployment.component_topology;
    let spec = "app".parse().expect("Component Spec ID");
    let admission = ComponentSpecAdmission {
        spec_hash: topology.get(&spec).expect("Component Spec").spec_hash,
        component_spec: spec,
        maximum_root_instances: 1,
    };
    let projected = topology
        .project_for_admissions(std::slice::from_ref(&admission))
        .expect("Root topology");
    let release_build_id =
        ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([7; 32]));
    let root_principal = principal(2);
    let store_principal = principal(3);
    let operator = principal(8);
    let placement = subnet(6);
    let bootstrap = DesiredFleetBootstrap {
        admission_identity_origin: None,
        admission: compile_fleet_admission_policy_template(vec![principal(9)], Vec::new())
            .expect("admission template"),
        app: AppId::from("init_wire_test"),
        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
        component_deployment_configuration: deployment,
        coordinator: "coordinator".to_string(),
        coordinator_subnet: subnet(1),
        fleet_id: FleetId::from_generated_bytes([5; 32]),
        fresh_estate: false,
        release_build_id,
        root_funding: None,
        roots: vec![DesiredFleetBootstrapRoot {
            canister_pool_imports: vec!["pool-0".to_string()],
            component_admissions: vec![admission.clone()],
            component_topology_digest: projected.digest().expect("Root topology digest"),
            funding: crate::test_support::fleet_subnet_root_funding_authority(),
            limits: limits(),
            placement_subnet: placement,
            root: "root-0".to_string(),
            store: "store-0".to_string(),
        }],
    };
    let registry = registry_authority(&bootstrap, principal(1));
    let root_binding = FleetSubnetRootBinding {
        authority: registry.clone(),
        placement_subnet: placement,
        fleet_subnet_root: root_principal,
        component_admissions: vec![admission],
        component_topology_digest: bootstrap.roots[0].component_topology_digest,
        limits: bootstrap.roots[0].limits.clone(),
        funding: bootstrap.roots[0].funding.clone(),
    };
    let root_authority = FleetSubnetRootAuthority {
        binding: root_binding,
        initial_release_set: FleetSubnetRootReleaseSet {
            release_build_id,
            manifest_digest: ReleaseSetDigest::from_bytes([10; 32]),
        },
        expected_module_hash: [11; 32],
        wasm_store_authority: FleetSubnetWasmStoreAuthority {
            authority: registry.clone(),
            placement_subnet: placement,
            fleet_subnet_root: root_principal,
            wasm_store: store_principal,
            installation_controller: operator,
            release_build_id,
            wasm_module_hash: [12; 32],
        },
    };
    InitFixture {
        bootstrap,
        registry,
        root_authority,
    }
}

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte])
}

fn subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(principal(byte))
}

fn limits() -> FleetSubnetRootLimits {
    FleetSubnetRootLimits {
        maximum_component_instances: 1,
        maximum_registry_bytes: 16_777_216,
        maximum_wasm_store_bytes: 40_000_000,
        maximum_group_placements: 1,
        canister_pool: FleetSubnetCanisterPoolConfig {
            minimum_size: 2,
            maximum_size: 2,
            canister_cycles: Cycles::new(5_000_000_000_000),
            creation_execution_margin: Cycles::new(1_000_000_000_000),
        },
        cycles_funding: CyclesFundingBudget {
            window_secs: 3_600,
            maximum_cycles: Cycles::new(15_000_000_000_000),
        },
    }
}
