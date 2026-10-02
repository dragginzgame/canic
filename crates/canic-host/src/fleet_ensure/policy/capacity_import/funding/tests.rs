//! Additional funding keeps earlier debit and original custody in the same approval equation.

use super::*;
use crate::fleet_ensure::{
    model::capacity_import::survey::CapacityImportSampleRecord,
    policy::capacity_import::{
        tests::{plan, principal},
        validate_plan,
    },
};

fn funded() -> CapacityImportPlanRecord {
    let mut plan = plan();
    let source = &mut plan.sources[0];
    let before = CapacityImportSampleRecord {
        binding: source.binding.clone(),
        cycles: source.observed_cycles,
        reserved_cycles: source.observed_reserved_cycles,
    };
    let mut observed = before.clone();
    observed.cycles += 80;
    plan.funding_credits
        .push(CapacityImportFundingCreditRecord {
            origin: CapacityImportFundingOrigin::Survey {
                request_sha256: [7; 32],
            },
            before,
            observed,
            credited_cycles: 100,
        });
    source.observed_cycles += 100;
    plan
}

#[test]
fn additional_credit_preserves_prior_debit_and_native_floor() {
    let mut plan = funded();
    validate_plan(&plan).unwrap();
    assert_eq!(plan.sources[0].maximum_debit_cycles, 200);
    plan.funding_credits[0].observed.cycles = 899;
    assert!(matches!(
        validate_plan(&plan),
        Err(CapacityImportPolicyError::ConservationUnproven { .. })
    ));
    plan.funding_credits[0].observed.cycles = 799;
    plan.funding_credits[0].observed.reserved_cycles = 201;
    assert_eq!(
        validate_plan(&plan),
        Err(CapacityImportPolicyError::InsufficientCycles {
            canister: principal(9),
            required_cycles: 800,
            available_cycles: 799,
            shortfall_cycles: 1,
        })
    );
}

#[test]
fn credit_rejects_custody_changes_and_fictional_baselines() {
    let original = funded();
    let mut changed = original.clone();
    changed.funding_credits[0].observed.binding.controllers = vec![principal(99)];
    assert!(matches!(
        validate_plan(&changed),
        Err(CapacityImportPolicyError::SourceChanged { .. })
    ));
    let mut changed = original.clone();
    changed.funding_credits[0].observed.cycles = 1_101;
    validate_plan(&changed).unwrap();
    let mut changed = original.clone();
    changed.sources[0].observed_cycles -= 1;
    assert!(matches!(
        validate_plan(&changed),
        Err(CapacityImportPolicyError::ConservationUnproven { .. })
    ));
    let mut changed = original.clone();
    changed
        .funding_credits
        .push(changed.funding_credits[0].clone());
    assert_eq!(
        validate_plan(&changed),
        Err(CapacityImportPolicyError::InvalidSources)
    );
    let mut changed = original;
    changed.funding_credits[0].credited_cycles = 0;
    assert_eq!(
        validate_plan(&changed),
        Err(CapacityImportPolicyError::InvalidCycleBounds)
    );
}

#[test]
fn root_credit_keeps_root_debit_and_paid_call_limits() {
    let mut plan = plan();
    let budget = plan.root_budget;
    let mut binding = plan.sources[0].binding.clone();
    binding.canister_id = plan.authority.root;
    let before = CapacityImportSampleRecord {
        binding,
        cycles: budget.observed_cycles,
        reserved_cycles: budget.observed_reserved_cycles,
    };
    let mut observed = before.clone();
    observed.cycles += 80;
    plan.root_budget.observed_cycles += 100;
    plan.funding_credits
        .push(CapacityImportFundingCreditRecord {
            origin: CapacityImportFundingOrigin::BootstrapTerminal {
                plan_sha256: "07".repeat(32),
            },
            before,
            observed,
            credited_cycles: 100,
        });
    validate_plan(&plan).unwrap();
    assert_eq!(
        plan.root_budget.maximum_debit_cycles,
        budget.maximum_debit_cycles
    );
    assert_eq!(
        plan.root_budget.maximum_paid_calls,
        budget.maximum_paid_calls
    );
    plan.funding_credits[0].observed.cycles = budget.observed_cycles - 1;
    assert!(matches!(
        validate_plan(&plan),
        Err(CapacityImportPolicyError::ConservationUnproven { .. })
    ));
}

#[test]
fn only_running_root_custody_can_advance_version_between_funding_observations() {
    let mut plan = funded();
    let root = plan.authority.root;
    let credit = &mut plan.funding_credits[0];
    credit.before.binding.stopped = false;
    credit.before.binding.controllers = vec![root];
    credit.observed.binding = credit.before.binding.clone();
    credit.observed.binding.canister_version += 1;
    validate_observation(credit, root, 200, 800).unwrap();
    credit.origin = CapacityImportFundingOrigin::BootstrapSource {
        plan_sha256: "07".repeat(32),
    };
    assert!(matches!(
        validate_observation(credit, root, 200, 800),
        Err(CapacityImportPolicyError::SourceChanged { .. })
    ));
    credit.origin = CapacityImportFundingOrigin::Survey {
        request_sha256: [7; 32],
    };
    credit.before.binding.stopped = true;
    credit.observed.binding.stopped = true;
    assert!(matches!(
        validate_observation(credit, root, 200, 800),
        Err(CapacityImportPolicyError::SourceChanged { .. })
    ));
}
