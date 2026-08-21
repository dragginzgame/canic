//! Shared explicit protected-authority fixtures for control-plane unit tests.

use canic_core::{
    cdk::types::Cycles,
    ids::{
        CyclesFundingBudget, FleetCoordinatorRootFundingPolicy, FleetSubnetRootFundingAuthority,
        FleetSubnetRootFundingPolicy,
    },
};

pub(crate) fn coordinator_root_funding_policy() -> FleetCoordinatorRootFundingPolicy {
    FleetCoordinatorRootFundingPolicy {
        minimum_reserve_cycles: Cycles::new(100_000_000),
        budget: CyclesFundingBudget {
            window_secs: 3_600,
            maximum_cycles: Cycles::new(10_000_000_000_000),
        },
    }
}

pub(crate) fn fleet_subnet_root_funding_authority() -> FleetSubnetRootFundingAuthority {
    FleetSubnetRootFundingAuthority {
        root_funding: FleetSubnetRootFundingPolicy {
            request_threshold: Cycles::new(50_000_000_000),
            target_balance: Cycles::new(2_000_000_000_000),
            cooldown_secs: 300,
            budget: CyclesFundingBudget {
                window_secs: 3_600,
                maximum_cycles: Cycles::new(10_000_000_000_000),
            },
        },
        icp_refill: None,
    }
}
