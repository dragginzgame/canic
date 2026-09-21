//! Module: fleet_ensure::ops::reinstall::terminal
//!
//! Responsibility: bind completed receipts and phase evidence without decoding an executable source plan.
//! Does not own: retirement admission, remote observations, replacement or replay.
//! Boundary: opaque forecast fields remain source bytes; only supported completed effects are inspected.

use crate::fleet_ensure::{
    model::{
        CanisterPlan, DesiredCanisterInit, EffectState, EnsureAction, FleetEnsureCompletion,
        FleetEnsureContinuationAuthority, FleetEnsurePlanScope, FleetReinstallSourceRecord,
        FleetTerminalSourceRecord, MAX_FLEET_ENSURE_CANISTERS, MAX_FLEET_ENSURE_PROTOCOL_STEPS,
    },
    ops::{
        EnsurePaths, EnsureStateError, action_sha256, is_sha256, plan_content, read_document_bytes,
        read_journal, read_state,
    },
    policy::expected_plan_sha256,
    view::terminal_source::TerminalSourceView,
};
use canic_core::cdk::utils::hash::sha256_hex;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// Inspect completed evidence only. The original plan is never reconstructed or executed.
pub(in crate::fleet_ensure) fn read(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
) -> Result<TerminalSourceView, EnsureStateError> {
    let plan_bytes = bytes(&paths.plan)?;
    let journal_bytes = bytes(&paths.journal)?;
    let state_bytes = bytes(&paths.state)?;
    let mut raw: Value = serde_json::from_slice(&plan_bytes).map_err(|_| invalid())?;
    plan_content::hydrate(paths, &mut raw)?;
    let journal = read_journal(paths)?.ok_or_else(invalid)?;
    let state = read_state(paths, fleet)?;
    let operation_id: String = field(&raw, "operation_id")?;
    let plan_sha256: String = field(&raw, "plan_sha256")?;
    let identity = [
        field::<u16>(&raw, "schema_version")? == 1,
        field::<String>(&raw, "scope")? == "full",
        field::<String>(&raw, "environment")? == environment,
        field::<String>(&raw, "fleet")? == fleet,
        journal.fleet == fleet,
        journal.operation_id == operation_id,
        journal.plan_sha256 == plan_sha256,
        journal.completion == FleetEnsureCompletion::Converged,
        is_sha256(&operation_id),
        is_sha256(&plan_sha256),
        state.active_registry.is_some(),
        state.pending_principals.is_empty(),
        journal.funding_reviews.is_empty(),
        journal.estate_funding_required.is_none(),
        raw.get("reinstall") == Some(&Value::Null),
        raw.get("root_start_authority") == Some(&Value::Null),
        raw.get("terminal_inventory_operation_id") == Some(&Value::Null),
        field::<Vec<Value>>(&raw, "root_reinstall_bindings")?.is_empty(),
        field::<Vec<Value>>(&raw, "protocol_actions")?.is_empty(),
    ];
    if !identity.into_iter().all(|valid| valid) {
        return Err(invalid());
    }
    let mut source = TerminalSourceView {
        documents: FleetTerminalSourceRecord {
            operation_id,
            plan_sha256,
            plan_document_sha256: sha256_hex(&plan_bytes),
            journal_document_sha256: sha256_hex(&journal_bytes),
            state_document_sha256: sha256_hex(&state_bytes),
            phase_document_sha256: BTreeMap::new(),
        },
        reviewed_desired: field(&raw, "reviewed_desired")?,
        conservation: field(&raw, "conservation")?,
        actions: initial_actions(&raw)?,
        journal,
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
        &source.journal.funding_observations,
    )
    .map_err(|_| invalid())?;
    source.conservation.maximum_execution_burn_cycles = source
        .conservation
        .maximum_execution_burn_cycles
        .checked_add(allowance)
        .ok_or_else(invalid)?;
    phases(paths, &raw, &mut source)?;
    receipts(&source)?;
    Ok(source)
}

fn initial_actions(raw: &Value) -> Result<Vec<EnsureAction>, EnsureStateError> {
    let canisters: Vec<CanisterPlan> = field(raw, "canisters")?;
    if canisters.is_empty() || canisters.len() > MAX_FLEET_ENSURE_CANISTERS {
        return Err(invalid());
    }
    let mut actions = Vec::new();
    let mut names = BTreeSet::new();
    for canister in canisters {
        if !names.insert(canister.name.clone()) {
            return Err(invalid());
        }
        for action in canister.actions {
            let (EnsureAction::Fund {
                principal,
                pool_funding: None,
                ..
            }
            | EnsureAction::Install { principal, .. }) = &action
            else {
                return Err(invalid());
            };
            if action.name() != canister.name || canister.principal.as_ref() != Some(principal) {
                return Err(invalid());
            }
            let rank = match &action {
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
            actions.push((rank, action));
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
    raw: &Value,
    source: &mut TerminalSourceView,
) -> Result<(), EnsureStateError> {
    let authority: FleetEnsureContinuationAuthority = field(raw, "continuation")?;
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
            phase.desired_sha256 == field::<String>(raw, "desired_sha256")?,
            phase.planned_at_time == field::<u64>(raw, "planned_at_time")?,
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
            .insert(record.plan_sha256.clone(), sha256_hex(&bytes(&path)?))
            .is_some()
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
