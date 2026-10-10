use crate::{cycles::Cycles, ids::*};
use candid::Principal;

pub fn fleet_key(byte: u8) -> FleetKey {
    FleetKey {
        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
        fleet_id: FleetId::from_generated_bytes([byte.saturating_add(1); 32]),
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

/// Passive projection fixture for codec round trips, without policy admission.
pub fn fleet_admission_projection(target: ManagedCanisterBinding) -> FleetAdmissionProjection {
    let authority = match &target {
        ManagedCanisterBinding::Component(binding) => binding.authority.binding.clone(),
        ManagedCanisterBinding::ComponentChild(binding) => {
            binding.component.authority.binding.clone()
        }
    };
    FleetAdmissionProjection {
        schema_version: 1,
        authority,
        target,
        generation: 1,
        policy_digest: [1; 32],
        projection_digest: [2; 32],
        principals: vec![Principal::from_slice(&[1; 29])],
    }
}
