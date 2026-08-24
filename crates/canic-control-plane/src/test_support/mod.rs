//! Shared explicit protected-authority fixtures for control-plane unit tests.

use candid::Principal;
use canic_core::{
    cdk::types::Cycles,
    dto::{
        fleet_funding::{
            FleetRootFundingAcceptanceReceipt, FleetRootFundingAcceptanceRequest,
            FleetRootFundingRequest,
        },
        fleet_registry::FleetRegistryVersion,
    },
    ids::{
        AppId, CanonicalNetworkId, CyclesFundingBudget, FleetAdmissionPolicy, FleetBinding,
        FleetCoordinatorBinding, FleetCoordinatorRootFundingPolicy, FleetFundingProfile, FleetId,
        FleetKey, FleetRegistryAuthority, FleetSubnetRootFundingAuthority,
        FleetSubnetRootFundingPolicy, SubnetId,
    },
    shared_support::fleet_admission_policy::{
        bind_initial_fleet_admission_policy, compile_fleet_admission_policy_template,
    },
    shared_support::fleet_funding_policy::{
        fleet_root_funding_operation_id, fleet_subnet_root_funding_policy_hash,
    },
};

pub fn coordinator_root_funding_policy() -> FleetCoordinatorRootFundingPolicy {
    FleetCoordinatorRootFundingPolicy {
        funding_profile: FleetFundingProfile::SingleSubnet,
        minimum_reserve_cycles: Cycles::new(30_000_000_000_000),
        budget: CyclesFundingBudget {
            window_secs: 90 * 24 * 60 * 60,
            maximum_cycles: Cycles::new(30_000_000_000_000),
        },
        maximum_automatic_grants: 4,
        maximum_automatic_cycles: Cycles::new(120_000_000_000_000),
    }
}

pub fn fleet_subnet_root_funding_authority() -> FleetSubnetRootFundingAuthority {
    FleetSubnetRootFundingAuthority {
        root_funding: FleetSubnetRootFundingPolicy {
            funding_profile: FleetFundingProfile::SingleSubnet,
            request_threshold: Cycles::new(10_000_000_000_000),
            target_balance: Cycles::new(30_000_000_000_000),
            cooldown_secs: 30 * 24 * 60 * 60,
            budget: CyclesFundingBudget {
                window_secs: 90 * 24 * 60 * 60,
                maximum_cycles: Cycles::new(30_000_000_000_000),
            },
            maximum_automatic_grants: 4,
            maximum_automatic_cycles: Cycles::new(120_000_000_000_000),
        },
        icp_refill: None,
    }
}

pub fn fleet_admission_policy(fleet: FleetBinding) -> FleetAdmissionPolicy {
    let template =
        compile_fleet_admission_policy_template(vec![Principal::from_slice(&[1; 29])], Vec::new())
            .expect("test Fleet admission template");
    bind_initial_fleet_admission_policy(fleet, &template).expect("test Fleet admission policy")
}

pub fn root_funding_request_fixture(operation_sequence: u64) -> FleetRootFundingRequest {
    let coordinator = Principal::from_slice(&[70; 29]);
    let root = Principal::from_slice(&[71; 29]);
    let expected_registry = FleetRegistryVersion {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                        fleet_id: FleetId::from_generated_bytes([72; 32]),
                    },
                    app: AppId::from("root-funding-test"),
                },
                coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[73; 29])),
                coordinator,
            },
            epoch: 1,
        },
        revision: 2,
        content_hash: [74; 32],
    };
    let funding = fleet_subnet_root_funding_authority();
    let policy_hash = fleet_subnet_root_funding_policy_hash(&funding);
    let observed_balance = 42_200_000_000;
    let requested_cycles = funding.root_funding.target_balance.to_u128() - observed_balance;
    FleetRootFundingRequest {
        operation_id: fleet_root_funding_operation_id(
            coordinator,
            root,
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

pub fn root_funding_acceptance_receipt_fixture(
    request: &FleetRootFundingRequest,
    accepted_at_ns: u64,
) -> FleetRootFundingAcceptanceReceipt {
    FleetRootFundingAcceptanceReceipt {
        request: FleetRootFundingAcceptanceRequest {
            operation_id: request.operation_id,
            operation_sequence: request.operation_sequence,
            expected_registry: request.expected_registry.clone(),
            observed_balance: request.observed_balance.clone(),
            granted_cycles: request.requested_cycles.clone(),
            policy_hash: request.policy_hash,
        },
        fleet_subnet_root: Principal::from_slice(&[71; 29]),
        coordinator: Principal::from_slice(&[70; 29]),
        accepted_at_ns,
    }
}
