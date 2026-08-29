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
package = "root"

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
        },
        cycles_funding: CyclesFundingBudget {
            window_secs: 3_600,
            maximum_cycles: Cycles::new(15_000_000_000_000),
        },
    }
}
