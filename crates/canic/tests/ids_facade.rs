// Category C - Public facade data contracts without canister installation.

use candid::{Principal, decode_one, encode_one};
use canic::{
    dto::{
        abi::v1::{CanisterInitAuthority, CanisterInitPayload},
        component_deployment::ProtectedComponentDeployment,
        cycles::Cycles,
    },
    ids::{
        AppId, CanisterRole, CanonicalNetworkId, ComponentBinding, ComponentInstanceId,
        ComponentTopologyDigest, CyclesFundingBudget, FleetAdmissionProjection, FleetBinding,
        FleetCoordinatorBinding, FleetFundingProfile, FleetId, FleetKey, FleetRegistryAuthority,
        FleetSubnetCanisterPoolConfig, FleetSubnetRootBinding, FleetSubnetRootFundingAuthority,
        FleetSubnetRootFundingPolicy, FleetSubnetRootLimits, ManagedCanisterBinding,
        ReleaseBuildId, ReleaseBuildIdParseError, ReleaseBuildNonce, SubnetId,
    },
};

#[test]
fn managed_init_payload_is_constructible_through_public_facade_paths() {
    let release_build_id =
        ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([1; 32]));
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([2; 32]),
                },
                app: AppId::from("example"),
            },
            coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[3])),
            coordinator: Principal::from_slice(&[4]),
            recovery_controllers: Vec::new(),
        },
        epoch: 1,
    };
    let budget = CyclesFundingBudget {
        window_secs: 3_600,
        maximum_cycles: Cycles::new(100_000_000_000_000),
    };
    let root = FleetSubnetRootBinding {
        authority: authority.clone(),
        placement_subnet: SubnetId::from_principal(Principal::from_slice(&[5])),
        fleet_subnet_root: Principal::from_slice(&[6]),
        component_admissions: Vec::new(),
        component_topology_digest: ComponentTopologyDigest::from_bytes([7; 32]),
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 1,
            maximum_registry_bytes: 1_048_576,
            maximum_wasm_store_bytes: 16_777_216,
            maximum_group_placements: 1,
            canister_pool: FleetSubnetCanisterPoolConfig {
                minimum_size: 1,
                maximum_size: 2,
                canister_cycles: Cycles::new(5_000_000_000_000),
                creation_execution_margin: Cycles::new(1_000_000_000_000),
            },
            cycles_funding: budget.clone(),
        },
        funding: FleetSubnetRootFundingAuthority {
            root_funding: FleetSubnetRootFundingPolicy {
                funding_profile: FleetFundingProfile::SingleSubnet,
                request_threshold: Cycles::new(1_000_000_000_000),
                target_balance: Cycles::new(2_000_000_000_000),
                cooldown_secs: 60,
                budget,
                maximum_automatic_grants: 2,
                maximum_automatic_cycles: Cycles::new(100_000_000_000_000),
            },
            icp_refill: None,
        },
    };
    let binding = ComponentBinding {
        authority,
        component: ComponentInstanceId::from_generated_bytes([8; 32]),
        component_spec: "example".parse().expect("Component Spec"),
        spec_hash: [9; 32],
        role: CanisterRole::new("app"),
        placement_subnet: root.placement_subnet,
        fleet_subnet_root: root.fleet_subnet_root,
        canister_id: Principal::from_slice(&[10]),
    };
    let payload = CanisterInitPayload {
        root_install_id: [7; 32],
        component_install_id: [11; 32],
        fixture: None,
        install_id: [11; 32],
        release_build_id,
        authority: CanisterInitAuthority::Component {
            root,
            binding: binding.clone(),
        },
        component_deployment: Box::new(ProtectedComponentDeployment::UngroupedOrdinary {
            binding: binding.clone(),
        }),
        admission: Some(FleetAdmissionProjection {
            schema_version: 1,
            authority: binding.authority.binding.clone(),
            target: ManagedCanisterBinding::Component(binding),
            generation: 1,
            policy_digest: [12; 32],
            projection_digest: [13; 32],
            principals: vec![Principal::from_slice(&[14])],
        }),
    };
    let encoded = encode_one(&payload).expect("encode public managed payload");
    let decoded =
        decode_one::<CanisterInitPayload>(&encoded).expect("decode public managed payload");

    assert_eq!(decoded, payload);
    assert_caller_authority_contract(&payload);
}

fn assert_caller_authority_contract(payload: &CanisterInitPayload) {
    use canic::ids::{
        CallerComponentInstallation, CallerInstallation, CallerReceiverAuthority,
        CallerRootAuthority,
    };
    let CanisterInitAuthority::Component { root, binding } = &payload.authority else {
        unreachable!()
    };
    let receiver = CallerReceiverAuthority {
        receiver: CallerInstallation {
            binding: ManagedCanisterBinding::Component(binding.clone()),
            install_id: payload.install_id,
            component_install_id: payload.component_install_id,
        },
        issuer: CallerRootAuthority {
            registry: root.authority.clone(),
            root: root.fleet_subnet_root,
            install_id: payload.root_install_id,
        },
        policy_digest: [29; 32],
    };
    let component = CallerComponentInstallation {
        binding: binding.clone(),
        install_id: payload.component_install_id,
    };
    assert_eq!(
        decode_one::<CallerReceiverAuthority>(&encode_one(&receiver).unwrap()).unwrap(),
        receiver
    );
    assert_eq!(
        decode_one::<CallerComponentInstallation>(&encode_one(&component).unwrap()).unwrap(),
        component
    );
}

#[test]
fn release_build_construction_and_typed_parse_failures_are_public() {
    let id = ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([15; 32]));
    assert_eq!(id.to_string().parse::<ReleaseBuildId>(), Ok(id));
    assert_eq!(
        "invalid".parse::<ReleaseBuildId>(),
        Err(ReleaseBuildIdParseError::Length(7))
    );
    assert_eq!(
        "g".repeat(64).parse::<ReleaseBuildId>(),
        Err(ReleaseBuildIdParseError::CanonicalHex)
    );
}
