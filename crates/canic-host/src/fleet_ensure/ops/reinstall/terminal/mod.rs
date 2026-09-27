//! Module: fleet_ensure::ops::reinstall::terminal
//!
//! Responsibility: bind completed current-schema receipts and immutable phase evidence.
//! Does not own: retirement admission, remote observations, replacement or replay.
//! Boundary: current plan hashes and bounded completed effects remain authoritative.

pub(in crate::fleet_ensure) mod documents;
pub(in crate::fleet_ensure) mod inventory;
pub(in crate::fleet_ensure) mod receipt_audit;

use crate::fleet_ensure::{
    model::{
        CanisterPlan, DesiredCanisterInit, EffectState, EnsureAction, FleetEnsureCompletion,
        FleetEnsureJournalRecord, FleetEnsurePlan, FleetEnsurePlanScope,
        FleetReinstallSourceRecord, MAX_FLEET_ENSURE_CANISTERS, MAX_FLEET_ENSURE_PROTOCOL_STEPS,
    },
    ops::{
        EnsurePaths, EnsureStateError, action_sha256, continuation, is_sha256, plan_content,
        read_document_bytes,
    },
    policy::expected_plan_sha256,
    view::terminal_source::{TerminalJournalView, TerminalSourceView},
};
use canic_core::cdk::utils::hash::sha256_hex;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// Inspect current completed evidence without executing or rewriting the source.
pub(in crate::fleet_ensure) fn read(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
) -> Result<TerminalSourceView, EnsureStateError> {
    let documents = documents::read(paths, environment, fleet)?;
    let mut raw = documents.plan;
    plan_content::hydrate(paths, &mut raw)?;
    let plan: FleetEnsurePlan = serde_json::from_value(raw).map_err(|_| invalid())?;
    if expected_plan_sha256(&plan) != plan.plan_sha256 {
        return Err(invalid());
    }
    let journal = documents.journal;
    validate_journal_fields(&journal)?;
    let state: crate::fleet_ensure::model::FleetEnsureStateRecord =
        serde_json::from_value(documents.state).map_err(|_| invalid())?;
    let operation_id = &plan.operation_id;
    let plan_sha256 = &plan.plan_sha256;
    let identity = [
        plan.schema_version == 1,
        plan.scope == FleetEnsurePlanScope::Full,
        plan.environment == environment,
        plan.fleet == fleet,
        field::<String>(&journal, "fleet")? == fleet,
        field::<u16>(&journal, "schema_version")? == 1,
        &field::<String>(&journal, "operation_id")? == operation_id,
        &field::<String>(&journal, "plan_sha256")? == plan_sha256,
        field::<FleetEnsureCompletion>(&journal, "completion")? == FleetEnsureCompletion::Converged,
        is_sha256(operation_id),
        is_sha256(plan_sha256),
        state.active_registry.is_some(),
        state.pending_principals.is_empty(),
        field::<Vec<Value>>(&journal, "funding_reviews")?.is_empty(),
        journal.get("estate_funding_required") == Some(&Value::Null),
        plan.reinstall.is_none(),
        plan.infrastructure_bootstrap.is_none(),
        plan.root_start_authority.is_none(),
        plan.terminal_inventory_operation_id.is_none(),
        plan.root_reinstall_bindings.is_empty(),
        plan.protocol_actions.is_empty(),
    ];
    if !identity.into_iter().all(|valid| valid) {
        return Err(invalid());
    }
    let mut source = TerminalSourceView {
        planned_at_time: plan.planned_at_time,
        documents: documents.bindings,
        reviewed_desired: *plan.reviewed_desired.clone().ok_or_else(invalid)?,
        conservation: plan.conservation.clone(),
        actions: initial_actions(&plan.canisters)?,
        journal: journal_evidence(paths, &journal)?,
    };
    if source.reviewed_desired.desired().environment != environment
        || source.reviewed_desired.desired().fleet != fleet
    {
        return Err(invalid());
    }
    let allowance = crate::fleet_ensure::ops::funding_observation::validation::source_allowance(
        &crate::fleet_ensure::ops::funding_observation::resolved_from_state(
            source.reviewed_desired.desired(),
            &state,
        ),
        &source.documents.operation_id,
        &source.documents.plan_sha256,
        &field(&journal, "funding_observations")?,
    )
    .map_err(|_| invalid())?;
    source.conservation.maximum_execution_burn_cycles = source
        .conservation
        .maximum_execution_burn_cycles
        .checked_add(allowance)
        .ok_or_else(invalid)?;
    phases(paths, &plan, &mut source)?;
    receipts(&source)?;
    Ok(source)
}

fn validate_journal_fields(journal: &Value) -> Result<(), EnsureStateError> {
    serde_json::from_value::<FleetEnsureJournalRecord>(journal.clone()).map_err(|_| invalid())?;
    Ok(())
}

fn journal_evidence(
    paths: &EnsurePaths,
    raw: &Value,
) -> Result<TerminalJournalView, EnsureStateError> {
    let mut phases = field::<Vec<crate::fleet_ensure::model::FleetEnsureSuccessorPhaseRecord>>(
        raw,
        "successor_phases",
    )?;
    continuation::hydrate_phases(paths, &mut phases)?;
    let balances: BTreeMap<String, String> = field(raw, "initial_estate_funding_cycles_by_root")?;
    Ok(TerminalJournalView {
        effects: field(raw, "effects")?,
        successor_phases: phases,
        initial_controlled_cycles: field::<String>(raw, "initial_controlled_cycles")?
            .parse()
            .map_err(|_| invalid())?,
        initial_operator_cycles: field::<String>(raw, "initial_operator_cycles")?
            .parse()
            .map_err(|_| invalid())?,
        initial_estate_funding_cycles_by_root: balances
            .into_iter()
            .map(|(root, amount)| Ok((root, amount.parse().map_err(|_| invalid())?)))
            .collect::<Result<_, EnsureStateError>>()?,
    })
}

fn initial_actions(canisters: &[CanisterPlan]) -> Result<Vec<EnsureAction>, EnsureStateError> {
    if canisters.is_empty() || canisters.len() > MAX_FLEET_ENSURE_CANISTERS {
        return Err(invalid());
    }
    let mut actions = Vec::new();
    let mut names = BTreeSet::new();
    for canister in canisters {
        if !names.insert(canister.name.clone()) {
            return Err(invalid());
        }
        for action in &canister.actions {
            let (EnsureAction::Fund {
                principal,
                pool_funding: None,
                ..
            }
            | EnsureAction::Install { principal, .. }) = action
            else {
                return Err(invalid());
            };
            if action.name() != canister.name || canister.principal.as_ref() != Some(principal) {
                return Err(invalid());
            }
            let rank = match action {
                EnsureAction::Fund { .. } => 0,
                EnsureAction::Install {
                    canic_init: Some(DesiredCanisterInit::Coordinator),
                    ..
                } => 1,
                EnsureAction::Install {
                    canic_init: Some(DesiredCanisterInit::Store { .. }),
                    ..
                } => 2,
                EnsureAction::Install { .. } => 3,
                _ => return Err(invalid()),
            };
            actions.push((rank, action.clone()));
        }
    }
    if actions.len() > MAX_FLEET_ENSURE_PROTOCOL_STEPS {
        return Err(invalid());
    }
    actions.sort_by_key(|(rank, _)| *rank);
    Ok(actions.into_iter().map(|(_, action)| action).collect())
}

fn phases(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    source: &mut TerminalSourceView,
) -> Result<(), EnsureStateError> {
    let authority = plan.continuation.as_ref().ok_or_else(invalid)?;
    if source.journal.successor_phases.is_empty()
        || source.journal.successor_phases.len() > MAX_FLEET_ENSURE_PROTOCOL_STEPS
    {
        return Err(invalid());
    }
    let mut count = 0;
    let mut previous_burn = 0;
    for record in &source.journal.successor_phases {
        let phase = record.plan.as_deref().ok_or_else(invalid)?;
        let matches = [
            phase.plan_sha256 == record.plan_sha256,
            expected_plan_sha256(phase) == record.plan_sha256,
            phase.operation_id == source.documents.operation_id,
            phase.environment == source.reviewed_desired.desired().environment,
            phase.fleet == source.reviewed_desired.desired().fleet,
            phase.desired_sha256 == plan.desired_sha256,
            phase.planned_at_time == plan.planned_at_time,
            phase.reviewed_desired.as_deref() == Some(&source.reviewed_desired),
            phase.scope == FleetEnsurePlanScope::Full,
            phase.continuation.is_none(),
            phase.reinstall.is_none(),
            phase.root_start_authority.is_none(),
            phase.root_reinstall_bindings.is_empty(),
            phase
                .canisters
                .iter()
                .all(|canister| canister.actions.is_empty()),
            record.execution_burn_before_phase >= previous_burn,
            record.execution_burn_before_phase <= source.conservation.maximum_execution_burn_cycles,
            phase
                .protocol_actions
                .iter()
                .all(|a| matches!(a, EnsureAction::FleetProtocol { .. })),
        ];
        if !matches.into_iter().all(|valid| valid) {
            return Err(invalid());
        }
        previous_burn = record.execution_burn_before_phase;
        count += phase.protocol_actions.len();
        if count > authority.maximum_successor_actions as usize
            || count > MAX_FLEET_ENSURE_PROTOCOL_STEPS
        {
            return Err(invalid());
        }
        let path = paths
            .plan
            .with_file_name("phases")
            .join(format!("{}.json", record.plan_sha256));
        if source
            .documents
            .phase_document_sha256
            .get(&record.plan_sha256)
            != Some(&sha256_hex(&bytes(&path)?))
        {
            return Err(invalid());
        }
        source
            .actions
            .extend(phase.protocol_actions.iter().cloned());
    }
    Ok(())
}

fn receipts(source: &TerminalSourceView) -> Result<(), EnsureStateError> {
    if source.actions.is_empty() || source.actions.len() != source.journal.effects.len() {
        return Err(invalid());
    }
    let mut hashes = BTreeSet::new();
    for (action, effect) in source.actions.iter().zip(&source.journal.effects) {
        if effect.state != EffectState::Applied
            || effect.action_sha256 != action_sha256(action)
            || !hashes.insert(&effect.action_sha256)
            || !effect.attempts_match(action)
        {
            return Err(invalid());
        }
        let EnsureAction::Fund {
            amount,
            expected_post_cycles,
            funding_deficit_cycles,
            funding_margin_cycles,
            ..
        } = action
        else {
            continue;
        };
        let receipt_present = effect
            .receipt
            .as_ref()
            .is_some_and(|receipt| !receipt.is_empty());
        let payment_completed = crate::fleet_ensure::ops::native_funding_applied(
            crate::fleet_ensure::ops::NativeFundingObservation {
                amount: *amount,
                expected_post_cycles: *expected_post_cycles,
                funding_deficit_cycles: *funding_deficit_cycles,
                funding_margin_cycles: *funding_margin_cycles,
                live_cycles: effect.post_cycles,
                pre_cycles: effect.pre_cycles,
            },
        );
        if !(receipt_present && payment_completed) {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(in crate::fleet_ensure) fn capture(
    root: &Path,
    view: &TerminalSourceView,
    conservation: crate::fleet_ensure::model::ActualCycleConservation,
) -> Result<FleetReinstallSourceRecord, EnsureStateError> {
    let mut source = super::capture_source(root, view.reviewed_desired.desired())?;
    source.terminal_retirement = Some(Box::new(
        crate::fleet_ensure::model::FleetTerminalRetirementRecord {
            source: view.documents.clone(),
            conservation: crate::fleet_ensure::model::FleetRetirementConservationRecord::NetBalance(
                conservation,
            ),
        },
    ));
    Ok(source)
}

fn field<T: DeserializeOwned>(raw: &Value, key: &str) -> Result<T, EnsureStateError> {
    serde_json::from_value(raw.get(key).cloned().ok_or_else(invalid)?).map_err(|_| invalid())
}
fn bytes(path: &Path) -> Result<Vec<u8>, EnsureStateError> {
    read_document_bytes(path)?.ok_or_else(invalid)
}
const fn invalid() -> EnsureStateError {
    EnsureStateError::InvalidTerminalSource
}

/// Bind independently authenticated debit evidence without editing the source documents.
pub(in crate::fleet_ensure) fn capture_external_debit(
    mut source: FleetReinstallSourceRecord,
    debit: crate::fleet_ensure::model::RetirementWithdrawalRecord,
) -> Result<FleetReinstallSourceRecord, EnsureStateError> {
    use crate::fleet_ensure::model::{
        FleetRetirementConservationRecord, RetirementExternalDebitRecord,
    };
    let retirement = source.terminal_retirement.as_mut().ok_or_else(invalid)?;
    let FleetRetirementConservationRecord::NetBalance(conservation) = &retirement.conservation
    else {
        return Err(invalid());
    };
    retirement.conservation =
        FleetRetirementConservationRecord::ExternalDebit(Box::new(RetirementExternalDebitRecord {
            source_conservation: conservation.clone(),
            external_debit: debit,
        }));
    Ok(source)
}
