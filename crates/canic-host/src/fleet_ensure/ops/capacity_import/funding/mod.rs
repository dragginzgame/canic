//! Bind additional capacity funding to an original observation and a fresh protected sample.
//!
//! These conversions never transfer cycles or replace an original survey, plan or journal.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    dto::capacity_import::CapacityImportReviewRequest,
    model::{
        FleetEnsurePlan,
        capacity_import::{
            CapacityImportPlanRecord,
            funding::{CapacityImportFundingCreditRecord, CapacityImportFundingOrigin},
            survey::CapacityImportSampleRecord,
        },
        infrastructure_bootstrap::InfrastructureBootstrapTerminalRecord,
    },
    ops::{
        self, EnsurePaths,
        capacity_import::{
            admission::survey::CapacityImportSurveyStore,
            journal::{CapacityImportJournalError, CapacityImportJournalStore},
        },
    },
    policy::capacity_import::{CapacityImportPolicyError, funding},
    view::capacity_import::{CapacityImportFundingBaselineView, CapacityImportFundingSampleView},
};
use candid::Principal;
use std::collections::{BTreeMap, BTreeSet};

/// Reject unknown, duplicate and zero credits before reserving a management observation.
pub(in crate::fleet_ensure) fn requested(
    request: &CapacityImportReviewRequest,
    root: Principal,
) -> Result<BTreeMap<Principal, u128>, CapacityImportJournalError> {
    let selected = request
        .canisters
        .iter()
        .copied()
        .chain([root])
        .collect::<BTreeSet<_>>();
    let mut credits = BTreeMap::new();
    for credit in &request.funding_credits {
        if credit.cycles == 0
            || !selected.contains(&credit.canister)
            || credits.insert(credit.canister, credit.cycles).is_some()
        {
            return Err(CapacityImportPolicyError::InvalidSources.into());
        }
    }
    Ok(credits)
}

/// Bootstrap's original pool sample and terminal Root sample remain their accounting owners.
pub(in crate::fleet_ensure) fn bootstrap_baseline(
    plan: &FleetEnsurePlan,
    terminal: Option<&InfrastructureBootstrapTerminalRecord>,
    canister: Principal,
    root: Principal,
) -> Option<CapacityImportFundingBaselineView> {
    if canister == root {
        return terminal?
            .canisters
            .values()
            .find(|sample| sample.binding.canister_id == canister)
            .map(|sample| CapacityImportFundingBaselineView {
                sample: sample.clone(),
                origin: CapacityImportFundingOrigin::BootstrapTerminal {
                    plan_sha256: plan.plan_sha256.clone(),
                },
            });
    }
    plan.infrastructure_bootstrap
        .as_ref()?
        .sources
        .values()
        .find(|source| source.sample.binding.canister_id == canister)
        .map(|source| CapacityImportFundingBaselineView {
            sample: source.sample.clone(),
            origin: CapacityImportFundingOrigin::BootstrapSource {
                plan_sha256: plan.plan_sha256.clone(),
            },
        })
}

/// An explicit credit can use a previous unissued review's successful survey without resetting it.
pub(in crate::fleet_ensure) fn survey_baseline(
    owner: &CapacityImportJournalStore,
    paths: &EnsurePaths,
    request_sha256: [u8; 32],
    canisters: &[Principal],
    canister: Principal,
) -> Result<CapacityImportFundingBaselineView, CapacityImportJournalError> {
    let sample = CapacityImportSurveyStore::original_sample(
        owner,
        paths,
        request_sha256,
        canisters,
        canister,
    )?
    .ok_or(CapacityImportJournalError::FundingBaselineMissing { canister })?;
    Ok(CapacityImportFundingBaselineView {
        sample,
        origin: CapacityImportFundingOrigin::Survey { request_sha256 },
    })
}

/// Construct a credited baseline only after checking the complete original debit equation.
pub(in crate::fleet_ensure) fn recognize(
    baseline: CapacityImportFundingBaselineView,
    observed: CapacityImportSampleRecord,
    credited_cycles: u128,
    root: Principal,
    maximum_debit: u128,
    minimum: u128,
) -> Result<CapacityImportFundingSampleView, CapacityImportJournalError> {
    let credit = CapacityImportFundingCreditRecord {
        origin: baseline.origin,
        before: baseline.sample,
        observed,
        credited_cycles,
    };
    funding::validate_observation(&credit, root, maximum_debit, minimum)?;
    let sample = CapacityImportSampleRecord {
        binding: credit.observed.binding.clone(),
        cycles: funding::credited_cycles(&credit)?,
        reserved_cycles: credit.before.reserved_cycles,
    };
    Ok(CapacityImportFundingSampleView { sample, credit })
}

/// A bootstrap source keeps its original custody and balance plus only its sealed credit.
pub(in crate::fleet_ensure) fn bootstrap_source_matches(
    bootstrap: &FleetEnsurePlan,
    import: &CapacityImportPlanRecord,
    original: &CapacityImportSampleRecord,
    imported: &crate::fleet_ensure::model::capacity_import::CapacityImportSourceRecord,
) -> bool {
    let additional = match import
        .funding_credits
        .iter()
        .find(|credit| credit.before.binding.canister_id == original.binding.canister_id)
    {
        Some(credit) => {
            if credit.before != *original
                || !matches!(&credit.origin,
                CapacityImportFundingOrigin::BootstrapSource { plan_sha256 } if *plan_sha256 == bootstrap.plan_sha256)
            {
                return false;
            }
            credit.credited_cycles
        }
        None => 0,
    };
    imported.binding == original.binding
        && Some(imported.observed_cycles) == original.cycles.checked_add(additional)
        && imported.observed_reserved_cycles == original.reserved_cycles
}

/// Verify every original owner before an uncompleted import may issue another effect.
pub(in crate::fleet_ensure) fn verify_origins(
    owner: &CapacityImportJournalStore,
    paths: &EnsurePaths,
    plan: &CapacityImportPlanRecord,
) -> Result<(), CapacityImportJournalError> {
    if plan.funding_credits.is_empty() {
        return Ok(());
    }
    let canisters = plan
        .sources
        .iter()
        .map(|source| source.binding.canister_id)
        .chain([plan.authority.root])
        .collect::<Vec<_>>();
    for credit in &plan.funding_credits {
        let canister = credit.before.binding.canister_id;
        let baseline = match &credit.origin {
            CapacityImportFundingOrigin::Survey { request_sha256 } => {
                survey_baseline(owner, paths, *request_sha256, &canisters, canister)?
            }
            CapacityImportFundingOrigin::BootstrapSource { plan_sha256 }
            | CapacityImportFundingOrigin::BootstrapTerminal { plan_sha256 } => {
                let bootstrap = ops::read_plan(paths)?
                    .filter(|plan| plan.plan_sha256 == *plan_sha256)
                    .ok_or(CapacityImportJournalError::Integrity)?;
                let terminal = if matches!(
                    credit.origin,
                    CapacityImportFundingOrigin::BootstrapTerminal { .. }
                ) {
                    let state = ops::read_state(paths, &bootstrap.fleet)?;
                    let journal =
                        ops::read_journal(paths)?.ok_or(CapacityImportJournalError::Integrity)?;
                    ops::infrastructure_bootstrap::terminal::read_receipt(
                        paths, &bootstrap, &journal, &state,
                    )
                    .map_err(|_| CapacityImportJournalError::Integrity)?
                } else {
                    None
                };
                bootstrap_baseline(&bootstrap, terminal.as_ref(), canister, plan.authority.root)
                    .ok_or(CapacityImportJournalError::Integrity)?
            }
        };
        let original_sample_matches = baseline.sample == credit.before;
        let original_owner_matches = baseline.origin == credit.origin;
        if !original_sample_matches || !original_owner_matches {
            return Err(CapacityImportJournalError::Integrity);
        }
    }
    Ok(())
}
