//! Persist terminal reset accounting and replay its exact completed local evidence without IC calls.

use crate::fleet_ensure::{
    model::{
        ActualCycleConservation, FleetEnsureCompletion, FleetEnsureJournalRecord, FleetEnsurePlan,
        FleetObservation,
        completed_handoff::{
            COMPLETED_RESET_MAXIMUM_TERMINAL_OBSERVATIONS, CompletedResetAccountingIntentRecord,
            CompletedResetBalancesRecord, CompletedResetTerminalRecord,
        },
    },
    ops::{self, EnsurePaths, EnsureStateError},
    view::completed_reset::CompletedResetBalancesView,
};
use canic_core::cdk::utils::hash::sha256_hex;

/// Consume a durable attempt before any terminal management observation can be issued.
pub(in crate::fleet_ensure) fn consume(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
) -> Result<(), EnsureStateError> {
    let maximum = plan
        .reinstall
        .as_ref()
        .and_then(|intent| intent.completed_reset.as_ref())
        .ok_or_else(conflict)?
        .maximum_terminal_observations;
    if maximum != COMPLETED_RESET_MAXIMUM_TERMINAL_OBSERVATIONS {
        return Err(conflict());
    }
    let path = paths.plan.with_file_name("completed-reset-accounting.json");
    let mut intent = ops::read_current::<CompletedResetAccountingIntentRecord>(&path)?
        .unwrap_or_else(|| CompletedResetAccountingIntentRecord {
            schema_version: 1,
            operation_id: plan.operation_id.clone(),
            plan_sha256: plan.plan_sha256.clone(),
            attempts: 0,
        });
    if intent.schema_version != 1
        || intent.operation_id != plan.operation_id
        || intent.plan_sha256 != plan.plan_sha256
    {
        return Err(conflict());
    }
    if intent.attempts >= maximum {
        return Err(EnsureStateError::CompletedResetAccountingBudget);
    }
    intent.attempts += 1;
    ops::write_current(&path, &intent)
}

/// Replace native/account observations with the last paid snapshot; preserve protocol evidence.
pub(in crate::fleet_ensure) fn attach(
    observation: &mut FleetObservation,
    balances: &CompletedResetBalancesView,
) -> Result<(), EnsureStateError> {
    let mut remaining = balances
        .canisters
        .keys()
        .collect::<std::collections::BTreeSet<_>>();
    for live in observation
        .canisters
        .values_mut()
        .filter_map(Option::as_mut)
    {
        let balance = balances
            .canisters
            .get(&live.principal)
            .ok_or_else(conflict)?;
        if !remaining.remove(&live.principal) {
            return Err(conflict());
        }
        live.cycles = balance.native_cycles;
        live.status = balance.status;
    }
    for (principal, cycles) in &mut observation.additional_controlled_cycles {
        let balance = balances.canisters.get(principal).ok_or_else(conflict)?;
        if !remaining.remove(principal) {
            return Err(conflict());
        }
        *cycles = balance.native_cycles;
    }
    if !remaining.is_empty() {
        return Err(conflict());
    }
    for domain in observation.estate_funding_domains.values_mut() {
        let owner = domain.root_principal.as_ref().ok_or_else(conflict)?;
        domain.balance_cycles = Some(*balances.ledger.get(owner).ok_or_else(conflict)?);
    }
    observation.operator_cycles = balances.operator_cycles;
    Ok(())
}

pub(in crate::fleet_ensure) fn read(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
) -> Result<Option<ActualCycleConservation>, EnsureStateError> {
    if plan
        .reinstall
        .as_ref()
        .is_none_or(|intent| intent.completed_reset.is_none())
        || journal.completion != FleetEnsureCompletion::Converged
    {
        return Ok(None);
    }
    let path = paths.plan.with_file_name("completed-reset-terminal.json");
    let Some(receipt): Option<CompletedResetTerminalRecord> = ops::read_current(&path)? else {
        return Ok(None);
    };
    if receipt.schema_version != 1
        || receipt.operation_id != plan.operation_id
        || receipt.plan_sha256 != plan.plan_sha256
        || receipt.journal_document_sha256 != document(&paths.journal)?
        || receipt.state_document_sha256 != document(&paths.state)?
        || digest(&receipt)? != receipt.receipt_sha256
        || receipt.actual.observed_starting_cycles != journal.initial_controlled_cycles
    {
        return Err(conflict());
    }
    Ok(Some(receipt.actual))
}

pub(in crate::fleet_ensure) fn retain(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    actual: &ActualCycleConservation,
    balances: CompletedResetBalancesView,
) -> Result<(), EnsureStateError> {
    let mut record = CompletedResetTerminalRecord {
        schema_version: 1,
        operation_id: plan.operation_id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        journal_document_sha256: document(&paths.journal)?,
        state_document_sha256: document(&paths.state)?,
        actual: actual.clone(),
        balances: CompletedResetBalancesRecord {
            canisters: balances.canisters,
            ledger: balances.ledger,
            operator_cycles: balances.operator_cycles,
        },
        receipt_sha256: String::new(),
    };
    record.receipt_sha256 = digest(&record)?;
    ops::write_current(
        &paths.plan.with_file_name("completed-reset-terminal.json"),
        &record,
    )
}

fn digest(record: &CompletedResetTerminalRecord) -> Result<String, EnsureStateError> {
    let mut value = record.clone();
    value.receipt_sha256.clear();
    let bytes = crate::fleet_ensure::json::to_vec(&value).map_err(|_| conflict())?;
    Ok(sha256_hex(&bytes))
}
fn document(path: &std::path::Path) -> Result<String, EnsureStateError> {
    ops::read_document_bytes(path)?
        .map(|bytes| sha256_hex(&bytes))
        .ok_or_else(conflict)
}
const fn conflict() -> EnsureStateError {
    EnsureStateError::CompletedHandoffConflict
}
