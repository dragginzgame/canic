//! Repo-only PocketIC fixtures layered on top of `ic-testkit`.

use canic_core::{
    cdk::candid::Principal,
    cdk::types::Cycles,
    ids::{
        CyclesFundingBudget, FleetAdmissionPolicy, FleetBinding, FleetCoordinatorRootFundingPolicy,
        FleetFundingProfile, FleetSubnetRootFundingAuthority, FleetSubnetRootFundingPolicy,
    },
    shared_support::fleet_admission_policy::{
        bind_initial_fleet_admission_policy, compile_fleet_admission_policy_template,
    },
};
#[cfg(test)]
use std::sync::{Mutex, MutexGuard, PoisonError};
#[cfg(test)]
use std::{collections::BTreeSet, panic::AssertUnwindSafe, time::Instant};

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

#[cfg(test)]
type GovernedTestCase = (&'static str, fn());

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

pub(crate) const fn root_funding_authority() -> FleetSubnetRootFundingAuthority {
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

pub(crate) fn fleet_admission_policy(fleet: FleetBinding) -> FleetAdmissionPolicy {
    let template =
        compile_fleet_admission_policy_template(vec![Principal::from_slice(&[1; 29])], Vec::new())
            .expect("PocketIC Fleet admission template");
    bind_initial_fleet_admission_policy(fleet, &template).expect("PocketIC Fleet admission policy")
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

#[cfg(test)]
fn run_governed_test_cases(cases: Vec<GovernedTestCase>) {
    let mut failures = Vec::new();
    let mut timings = Vec::new();
    for (name, test) in cases {
        let started_at = Instant::now();
        eprintln!("[canic-testing-internal] START {name}");
        let failed = std::panic::catch_unwind(AssertUnwindSafe(test)).is_err();
        let elapsed = started_at.elapsed().as_secs_f64();
        timings.push((name, elapsed));
        if failed {
            eprintln!("[canic-testing-internal] FAIL {name} elapsed={elapsed:.3}s");
            failures.push(name);
        } else {
            eprintln!("[canic-testing-internal] PASS {name} elapsed={elapsed:.3}s");
        }
    }

    timings.sort_by(|left, right| right.1.total_cmp(&left.1));
    eprintln!("[canic-testing-internal] slowest governed cases:");
    for (name, elapsed) in timings.into_iter().take(10) {
        eprintln!("[canic-testing-internal] SLOW {name} elapsed={elapsed:.3}s");
    }

    assert!(
        failures.is_empty(),
        "governed internal test failures: {}",
        failures.join(", ")
    );
}

// -----------------------------------------------------------------------------
// Governed runner entry points
// -----------------------------------------------------------------------------

#[cfg(test)]
mod governed_suite {
    use super::*;

    #[test]
    #[ignore = "the workspace runner executes this fast tier explicitly"]
    fn governed_fast_internal_suite() {
        assert_governed_pocketic_order();
        let mut cases = artifacts::governed_fast_cases();
        cases.extend(lifecycle::governed_fast_cases());
        cases.extend(root::governed_fast_cases());
        assert_eq!(cases.len(), 5);
        run_governed_test_cases(cases);
    }

    #[test]
    #[ignore = "the workspace runner supplies one shared PocketIC server and serial process"]
    fn governed_serial_pocketic_suite() {
        assert_governed_pocketic_order();
        let mut cases = fleet_registry::governed_pocketic_cases();
        cases.extend(fleet_coordinator::governed_pocketic_cases());
        cases.extend(lifecycle::governed_pocketic_cases());
        run_governed_test_cases(cases);
    }

    fn assert_governed_pocketic_order() {
        let mut cases = fleet_registry::governed_pocketic_cases();
        cases.extend(fleet_coordinator::governed_pocketic_cases());
        cases.extend(lifecycle::governed_pocketic_cases());
        let names = cases.iter().map(|(name, _)| *name).collect::<Vec<_>>();
        assert_eq!(names.len(), 33);
        assert_eq!(names[0], "Fleet deployment restore");
        assert_eq!(names[1], "autonomous Root removal");
        assert_eq!(
            names.iter().copied().collect::<BTreeSet<_>>().len(),
            names.len()
        );
    }
}
