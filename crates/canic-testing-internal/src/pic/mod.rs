//! Repo-only PocketIC fixtures layered on top of `ic-testkit`.

use canic_core::{
    cdk::types::Cycles,
    ids::{
        CyclesFundingBudget, FleetCoordinatorRootFundingPolicy, FleetSubnetRootFundingAuthority,
        FleetSubnetRootFundingPolicy,
    },
};
#[cfg(test)]
use std::sync::{Mutex, MutexGuard, PoisonError};

mod artifacts;
mod audit;
mod canic;
mod delegation;
#[cfg(test)]
mod fleet_coordinator;
mod fleet_registry;
mod lifecycle;
mod root;
mod startup;

pub use artifacts::{CanicWasmBuildProfile, build_internal_test_wasm_canisters};
pub use audit::{
    RootAuditProbeFixture, install_audit_leaf_probe, install_audit_root_probe,
    install_audit_scaling_probe,
};
pub use canic::{
    CanicPicExt, install_standalone_canister, install_standalone_canister_on_pic,
    managed_test_init_identity, report_canister_diagnostics, report_canister_diagnostics_batch,
    wait_until_ready,
};
pub use delegation::{
    create_user_shard, issue_delegated_token_from_active_proof,
    issue_delegated_token_from_active_proof_with_request_nonce, role_grant,
};
pub use fleet_registry::{
    ActiveComponentRegistryFixture, setup_active_component_registry,
    setup_fresh_active_component_registry,
};
pub use lifecycle::{
    CanicIcydbLifecycleFixture, LifecycleBoundaryFixture, UninstalledCanicFixture,
    icydb_participant_trap_wasm, install_canic_icydb_lifecycle_fixture,
    install_lifecycle_boundary_fixture, invalid_init_args, lifecycle_participant_init_trap_wasm,
    lifecycle_participant_trap_wasm, upgrade_args,
};
pub use root::{
    RootBaselineMetadata, RootBaselineRecipe, RootBaselineRecipeError, RootBaselineSpec,
    build_root_cached_baseline, ensure_root_release_artifacts_built, load_root_wasm,
    restore_root_cached_baseline, setup_root_topology,
};
pub use startup::start_pocket_ic;

pub(super) const SNAPSHOT_RESTORE_MINIMUM_CYCLES: u128 = 200_000_000_000_000;

pub(crate) const fn coordinator_root_funding_policy() -> FleetCoordinatorRootFundingPolicy {
    FleetCoordinatorRootFundingPolicy {
        minimum_reserve_cycles: Cycles::new(100_000_000),
        budget: CyclesFundingBudget {
            window_secs: 3_600,
            maximum_cycles: Cycles::new(10_000_000_000_000),
        },
    }
}

pub(crate) const fn root_funding_authority() -> FleetSubnetRootFundingAuthority {
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

#[cfg(test)]
static PIC_UNIT_TEST_SERIAL: Mutex<()> = Mutex::new(());

// Serialize the crate-local PocketIC unit journeys before they build artifacts or start a server.
#[cfg(test)]
fn acquire_pic_unit_test_serial_guard() -> MutexGuard<'static, ()> {
    PIC_UNIT_TEST_SERIAL
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}
