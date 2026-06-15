use super::super::*;
use super::{
    profile::authority_profile_overlap_findings,
    reconciliation::{
        observed_expected_by_plan, observed_pool_expected_by_plan,
        observed_unplanned_canister_action, observed_unplanned_pool_action,
        reconcile_expected_canister, reconcile_expected_pool_canister, record_authority_outcome,
    },
};

/// Build a dry-run authority reconciliation plan from the current deployment
/// truth check. The plan is observational; it does not mutate controller state.
#[must_use]
pub fn build_authority_reconciliation_plan(
    check: &DeploymentCheckV1,
) -> AuthorityReconciliationPlanV1 {
    let mut canister_actions = Vec::new();
    let mut automatic_actions = Vec::new();
    let mut hard_failures = authority_profile_overlap_findings(&check.plan);
    let mut external_actions_required = Vec::new();

    for expected in &check.plan.expected_canisters {
        let action = reconcile_expected_canister(&check.plan, &check.inventory, expected);
        record_authority_outcome(
            action,
            &mut canister_actions,
            &mut automatic_actions,
            &mut hard_failures,
            &mut external_actions_required,
        );
    }

    for expected in &check.plan.expected_pool {
        let action = reconcile_expected_pool_canister(&check.inventory, expected);
        record_authority_outcome(
            action,
            &mut canister_actions,
            &mut automatic_actions,
            &mut hard_failures,
            &mut external_actions_required,
        );
    }

    for observed in &check.inventory.observed_canisters {
        if observed_expected_by_plan(&check.plan, observed) {
            continue;
        }
        let action = observed_unplanned_canister_action(observed);
        record_authority_outcome(
            action,
            &mut canister_actions,
            &mut automatic_actions,
            &mut hard_failures,
            &mut external_actions_required,
        );
    }

    for observed in &check.inventory.observed_pool {
        if observed_pool_expected_by_plan(&check.plan, observed) {
            continue;
        }
        let action = observed_unplanned_pool_action(observed);
        record_authority_outcome(
            action,
            &mut canister_actions,
            &mut automatic_actions,
            &mut hard_failures,
            &mut external_actions_required,
        );
    }

    AuthorityReconciliationPlanV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_id: check.plan.plan_id.clone(),
        inventory_id: check.inventory.inventory_id.clone(),
        authority_profile_hash: check
            .plan
            .deployment_identity
            .authority_profile_hash
            .clone(),
        canister_actions,
        automatic_actions,
        hard_failures,
        external_actions_required,
    }
}
