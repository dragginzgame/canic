//! Persist final accounting before publishing completion, then replay without remote calls.

use crate::fleet_ensure::{
    model::{
        ActualCycleConservation, CycleConservation, EffectState, FleetEnsureCompletion,
        FleetEnsureJournalRecord, FleetEnsurePlan, FleetEnsurePlanScope, FleetEnsureStateRecord,
        clean_reinstall::CleanReinstallTerminalRecord,
    },
    ops::{EnsurePaths, EnsureStateError, read_current, write_current},
};
use serde::Serialize;
use sha2_host::{Digest, Sha256};
use std::path::PathBuf;

fn path(paths: &EnsurePaths, plan: &FleetEnsurePlan) -> PathBuf {
    paths
        .plan
        .with_file_name("clean-reinstall-receipts")
        .join(format!("{}.json", plan.plan_sha256))
}

fn digest(value: &impl Serialize) -> Result<[u8; 32], EnsureStateError> {
    Ok(Sha256::digest(
        serde_json::to_vec(value).map_err(|_| EnsureStateError::InvalidTerminalSource)?,
    )
    .into())
}

fn journal_digest(journal: &FleetEnsureJournalRecord) -> Result<[u8; 32], EnsureStateError> {
    let mut completed = journal.clone();
    completed.completion = FleetEnsureCompletion::Converged;
    completed.stalled_observations = 0;
    digest(&completed)
}

fn verify_effects(journal: &FleetEnsureJournalRecord) -> Result<(), EnsureStateError> {
    if !matches!(
        journal.completion,
        FleetEnsureCompletion::InProgress | FleetEnsureCompletion::Converged
    ) || journal
        .effects
        .iter()
        .any(|effect| effect.state != EffectState::Applied)
    {
        return Err(EnsureStateError::InvalidTerminalSource);
    }
    Ok(())
}

/// Commit the exact observation and accounting after terminal verification succeeds.
pub(in crate::fleet_ensure) fn retain(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    actual: &ActualCycleConservation,
    conservation: &CycleConservation,
) -> Result<(), EnsureStateError> {
    if plan.scope != FleetEnsurePlanScope::Full {
        return Ok(());
    }
    let Some(selection) = super::read(paths)? else {
        return Ok(());
    };
    verify_effects(journal)?;
    verify_accounting(conservation, journal, actual)?;
    let receipt = CleanReinstallTerminalRecord {
        schema_version: 1,
        plan_sha256: plan.plan_sha256.clone(),
        journal_sha256: journal_digest(journal)?,
        state_sha256: digest(state)?,
        selection_sha256: digest(&selection)?,
        actual: actual.clone(),
    };
    let destination = path(paths, plan);
    if let Some(existing) = read_current::<CleanReinstallTerminalRecord>(&destination)? {
        if existing != receipt {
            return Err(EnsureStateError::InvalidTerminalSource);
        }
        return Ok(());
    }
    write_current(&destination, &receipt)
}

/// Recover completion using exact durable state before resolving identity or contacting the IC.
pub(in crate::fleet_ensure) fn read(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    conservation: &CycleConservation,
) -> Result<Option<ActualCycleConservation>, EnsureStateError> {
    if plan.scope != FleetEnsurePlanScope::Full {
        return Ok(None);
    }
    let Some(receipt) = read_current::<CleanReinstallTerminalRecord>(&path(paths, plan))? else {
        return Ok(None);
    };
    let selection = super::read(paths)?.ok_or(EnsureStateError::InvalidTerminalSource)?;
    if receipt.schema_version != 1
        || receipt.plan_sha256 != plan.plan_sha256
        || receipt.journal_sha256 != journal_digest(journal)?
        || receipt.state_sha256 != digest(state)?
        || receipt.selection_sha256 != digest(&selection)?
    {
        return Err(EnsureStateError::InvalidTerminalSource);
    }
    verify_effects(journal)?;
    verify_accounting(conservation, journal, &receipt.actual)?;
    Ok(Some(receipt.actual))
}

fn verify_accounting(
    reviewed: &CycleConservation,
    journal: &FleetEnsureJournalRecord,
    actual: &ActualCycleConservation,
) -> Result<(), EnsureStateError> {
    let invalid = || EnsureStateError::InvalidTerminalSource;
    let available = journal
        .initial_controlled_cycles
        .checked_add(actual.received_new_funding_cycles)
        .and_then(|cycles| cycles.checked_sub(actual.exact_estate_creation_fee_cycles))
        .ok_or_else(invalid)?;
    let maximum_estate_fees = reviewed
        .estate_funding_domains
        .iter()
        .try_fold(0_u128, |total, domain| {
            total.checked_add(domain.maximum_creation_fee_cycles)
        })
        .ok_or_else(invalid)?;
    let maximum_estate_funding = reviewed
        .estate_funding_domains
        .iter()
        .try_fold(0_u128, |total, domain| {
            total.checked_add(domain.maximum_funding_cycles)
        })
        .ok_or_else(invalid)?;
    let funding_matches = actual.observed_starting_cycles == journal.initial_controlled_cycles
        && actual.exact_unavoidable_fee_cycles == reviewed.maximum_unavoidable_fee_cycles
        && actual
            .received_new_funding_cycles
            .checked_add(actual.exact_unavoidable_fee_cycles)
            == Some(actual.operator_debit_cycles);
    let reviewed_bounds_hold = actual.operator_debit_cycles
        <= reviewed.maximum_operator_debit_cycles
        && actual.received_new_funding_cycles <= reviewed.maximum_new_funding_cycles
        && actual.exact_estate_creation_fee_cycles <= maximum_estate_fees
        && actual.estate_funding_cycles <= maximum_estate_funding
        && actual.observed_net_cycle_debit_cycles <= reviewed.maximum_execution_burn_cycles;
    let balance_matches = actual.observed_net_cycle_debit_cycles
        == available.saturating_sub(actual.final_controlled_cycles)
        && actual.observed_net_cycle_credit_cycles
            == actual.final_controlled_cycles.saturating_sub(available);
    if !funding_matches || !reviewed_bounds_hold || !balance_matches {
        return Err(invalid());
    }
    Ok(())
}
