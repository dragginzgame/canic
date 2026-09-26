//! Module: fleet_ensure::ops::reinstall::terminal::receipt_audit
//!
//! Responsibility: verify completed historical plan/action hashes and original payment bounds.
//! Does not own: predecessor execution, live completion, authority conversion or handoff approval.
//! Boundary: historical data can produce audit facts only, never current executable actions.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::read_regular_bytes,
    fleet_ensure::{
        json,
        model::{
            DesiredCanisterInit, DesiredCanisterKind, EffectRecord, EffectState,
            FleetEnsurePlanScope, FleetEnsureSuccessorPhaseRecord, MAX_FLEET_ENSURE_CANISTERS,
            MAX_FLEET_ENSURE_PROTOCOL_STEPS,
        },
        ops::{
            EnsurePaths, EnsureStateError, NativeFundingObservation, native_funding_applied,
            plan_content,
        },
        view::terminal_source::{
            CompletedReceiptAuditView,
            receipt_evidence::{
                CompletedActionEvidence, CompletedCanisterEvidence, CompletedPhaseEvidence,
                CompletedProtocolEvidence, EvidenceDesiredFleet,
            },
        },
    },
};
use canic_core::cdk::utils::hash::{hex_bytes, sha256_hex};
use serde_json::Value;
use sha2_host::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

/// Verify the completed evidence shape whose generated authority has no recovery declaration.
/// This checks local retained receipts; fresh live authority/conservation are separate gates.
pub(in crate::fleet_ensure) fn inspect(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
) -> Result<CompletedReceiptAuditView, EnsureStateError> {
    let snapshot = super::documents::read(paths, environment, fleet)?;
    super::validate_journal_fields(&snapshot.journal)?;
    let mut raw = snapshot.plan;
    plan_content::hydrate(paths, &mut raw)?;
    let plan: CompletedPhaseEvidence = decode(raw)?;
    verify_hash(&plan)?;
    let desired = plan.reviewed_desired.as_ref().ok_or_else(invalid)?;
    let declaration = desired.desired.bootstrap.as_ref().ok_or_else(invalid)?;
    let source_complete = plan.scope == FleetEnsurePlanScope::Full
        && plan.continuation.is_some()
        && plan.protocol_actions.is_empty()
        && plan.reinstall.is_none()
        && plan.root_start_authority.is_none()
        && plan.root_reinstall_bindings.is_empty()
        && plan.terminal_inventory_operation_id.is_none();
    let claims_complete = snapshot.state.get("pending_principals") == Some(&serde_json::json!({}))
        && snapshot
            .state
            .get("active_registry")
            .is_some_and(Value::is_object)
        && snapshot.journal.get("estate_funding_required") == Some(&Value::Null)
        && snapshot.journal.get("funding_reviews") == Some(&serde_json::json!([]));
    // An absent observation adds no allowance. Paid-observation evidence needs its
    // own admission owner; this closed audit never discards or rebases it.
    let observations_empty = snapshot
        .journal
        .get("funding_observations")
        .is_none_or(|value| value == &serde_json::json!({}));
    if !source_complete
        || !claims_complete
        || !observations_empty
        || desired.desired.environment != environment
        || desired.desired.fleet != fleet
        || declaration.fresh_estate
    {
        return Err(invalid());
    }
    let mut actions = initial_actions(&plan.canisters)?;
    let phases: Vec<FleetEnsureSuccessorPhaseRecord> =
        field(&snapshot.journal, "successor_phases")?;
    if phases.is_empty() || phases.len() > MAX_FLEET_ENSURE_PROTOCOL_STEPS {
        return Err(invalid());
    }
    actions.extend(phase_actions(paths, &plan, &phases, &snapshot.bindings)?);
    let effects: Vec<EffectRecord> = field(&snapshot.journal, "effects")?;
    let funding = verify_receipts(&actions, &effects, &desired.desired.cycles_ledger)?;
    let fees = canic_core::cdk::types::Cycles::from_str(&desired.desired.ledger_fee_cycles)
        .map_err(|_| invalid())?
        .to_u128()
        .checked_mul(funding.payments)
        .ok_or_else(invalid)?;
    let debit = funding.cycles.checked_add(fees).ok_or_else(invalid)?;
    let bounds = &plan.conservation;
    let payment_bounds_match = funding.cycles == bounds.maximum_new_funding_cycles
        && fees == bounds.maximum_unavoidable_fee_cycles
        && debit == bounds.maximum_operator_debit_cycles
        && bounds.scheduled_transfer_cycles == 0;
    if !payment_bounds_match {
        return Err(invalid());
    }
    let initial_controlled_cycles = amount_field(&snapshot.journal, "initial_controlled_cycles")?;
    let initial_operator_cycles = amount_field(&snapshot.journal, "initial_operator_cycles")?;
    initial_controlled_cycles
        .checked_add(funding.cycles)
        .ok_or_else(invalid)?;
    initial_operator_cycles
        .checked_sub(debit)
        .ok_or_else(invalid)?;
    let initial_estate_funding_cycles_by_root =
        initial_root_accounts(&snapshot.journal, bounds, &desired.desired)?;
    Ok(CompletedReceiptAuditView {
        documents: snapshot.bindings,
        effect_count: effects.len(),
        phase_count: phases.len(),
        source_operator: desired.desired.operator.clone(),
        cycles_ledger: desired.desired.cycles_ledger.clone(),
        initial_controlled_cycles,
        initial_operator_cycles,
        initial_estate_funding_cycles_by_root,
        recorded_funding_cycles: funding.cycles,
        recorded_operator_debit_cycles: debit,
        original_maximum_execution_burn_cycles: bounds.maximum_execution_burn_cycles,
    })
}

fn amount_field(value: &Value, name: &str) -> Result<u128, EnsureStateError> {
    field::<String>(value, name)?.parse().map_err(|_| invalid())
}

/// Preserve all original Root accounts; this receipt lane admits no autonomous creation debit.
fn initial_root_accounts(
    journal: &Value,
    bounds: &crate::fleet_ensure::model::CycleConservation,
    desired: &EvidenceDesiredFleet,
) -> Result<BTreeMap<String, u128>, EnsureStateError> {
    let amounts: BTreeMap<String, String> =
        field(journal, "initial_estate_funding_cycles_by_root")?;
    let amounts = amounts
        .into_iter()
        .map(|(root, value)| {
            value
                .parse()
                .map(|amount| (root, amount))
                .map_err(|_| invalid())
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    if amounts.len() != bounds.estate_funding_domains.len() {
        return Err(invalid());
    }
    let mut roots = BTreeSet::new();
    for domain in &bounds.estate_funding_domains {
        let mut declarations = desired
            .canisters
            .iter()
            .filter(|canister| canister.name == domain.root);
        let root = declarations.next().ok_or_else(invalid)?;
        let account_matches = domain.cycles_ledger == desired.cycles_ledger
            && domain.root_principal.is_some()
            && domain.root_principal == root.principal
            && root.kind == DesiredCanisterKind::Root
            && declarations.next().is_none();
        let no_creation = domain.required_creation_count == 0
            && domain.maximum_creation_debit_cycles == 0
            && domain.maximum_creation_fee_cycles == 0
            && domain.maximum_funding_cycles == 0
            && domain.pending_creation_count == 0
            && domain.pending_creation.is_none();
        if !account_matches
            || !no_creation
            || !roots.insert(&domain.root)
            || domain.available_cycles.is_none()
            || amounts.get(&domain.root).copied() != domain.available_cycles
        {
            return Err(invalid());
        }
    }
    Ok(amounts)
}

fn phase_actions(
    paths: &EnsurePaths,
    plan: &CompletedPhaseEvidence,
    phases: &[FleetEnsureSuccessorPhaseRecord],
    bindings: &crate::fleet_ensure::model::FleetTerminalSourceRecord,
) -> Result<Vec<CompletedActionEvidence>, EnsureStateError> {
    let desired = plan.reviewed_desired.as_ref().ok_or_else(invalid)?;
    let authority = plan.continuation.as_ref().ok_or_else(invalid)?;
    let mut actions = Vec::new();
    let mut count = 0;
    let mut previous_burn = 0;
    for reference in phases {
        let path = paths
            .plan
            .with_file_name("phases")
            .join(format!("{}.json", reference.plan_sha256));
        let bytes = read_regular_bytes(&path, 32 * 1024 * 1024)
            .map_err(|source| EnsureStateError::Io { path, source })?;
        if bindings.phase_document_sha256.get(&reference.plan_sha256) != Some(&sha256_hex(&bytes)) {
            return Err(invalid());
        }
        let mut raw: Value = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
        plan_content::hydrate(paths, &mut raw)?;
        let phase: CompletedPhaseEvidence = decode(raw)?;
        verify_hash(&phase)?;
        let empty_authority = phase.scope == FleetEnsurePlanScope::Full
            && phase.continuation.is_none()
            && phase.reinstall.is_none()
            && phase.root_start_authority.is_none()
            && phase.root_reinstall_bindings.is_empty()
            && phase.terminal_inventory_operation_id.is_none()
            && phase
                .canisters
                .iter()
                .all(|canister| canister.actions.is_empty());
        if !empty_authority
            || phase.reviewed_desired.as_ref() != Some(desired)
            || reference.execution_burn_before_phase < previous_burn
            || reference.execution_burn_before_phase
                > plan.conservation.maximum_execution_burn_cycles
            || !phase
                .protocol_actions
                .iter()
                .all(|action| matches!(action, CompletedActionEvidence::FleetProtocol { .. }))
        {
            return Err(invalid());
        }
        previous_burn = reference.execution_burn_before_phase;
        count += phase.protocol_actions.len();
        if count > authority.maximum_successor_actions as usize
            || count > MAX_FLEET_ENSURE_PROTOCOL_STEPS
        {
            return Err(invalid());
        }
        actions.extend(phase.protocol_actions);
    }
    Ok(actions)
}

fn verify_hash(plan: &CompletedPhaseEvidence) -> Result<(), EnsureStateError> {
    if plan.plan_sha256 != phase_hash(plan)? {
        return Err(invalid());
    }
    Ok(())
}

fn phase_hash(plan: &CompletedPhaseEvidence) -> Result<String, EnsureStateError> {
    let mut canonical = plan.clone();
    canonical.plan_sha256.clear();
    let bytes = json::to_vec(&canonical).map_err(|_| invalid())?;
    let mut hash = Sha256::new();
    for field in [b"canic:fleet-ensure:plan:v1".as_slice(), bytes.as_slice()] {
        hash.update((field.len() as u64).to_be_bytes());
        hash.update(field);
    }
    Ok(hex_bytes(hash.finalize()))
}

fn initial_actions(
    canisters: &[CompletedCanisterEvidence],
) -> Result<Vec<CompletedActionEvidence>, EnsureStateError> {
    if canisters.is_empty() || canisters.len() > MAX_FLEET_ENSURE_CANISTERS {
        return Err(invalid());
    }
    let mut names = BTreeSet::new();
    let mut actions = Vec::new();
    for canister in canisters {
        if !names.insert(&canister.name) {
            return Err(invalid());
        }
        for action in &canister.actions {
            let (name, principal, rank) = match action {
                CompletedActionEvidence::Fund {
                    name, principal, ..
                } => (name, principal, 0),
                CompletedActionEvidence::Install {
                    name,
                    principal,
                    canic_init,
                    ..
                } => {
                    let rank = match canic_init {
                        Some(DesiredCanisterInit::Coordinator) => 1,
                        Some(DesiredCanisterInit::Store { .. }) => 2,
                        _ => 3,
                    };
                    (name, principal, rank)
                }
                CompletedActionEvidence::FleetProtocol { .. } => return Err(invalid()),
            };
            if name != &canister.name || canister.principal.as_ref() != Some(principal) {
                return Err(invalid());
            }
            actions.push((rank, action.clone()));
        }
    }
    if actions.len() > MAX_FLEET_ENSURE_PROTOCOL_STEPS {
        return Err(invalid());
    }
    actions.sort_by_key(|(rank, _)| *rank);
    Ok(actions.into_iter().map(|(_, action)| action).collect())
}

struct CompletedFunding {
    cycles: u128,
    payments: u128,
}

fn verify_receipts(
    actions: &[CompletedActionEvidence],
    effects: &[EffectRecord],
    ledger: &str,
) -> Result<CompletedFunding, EnsureStateError> {
    if actions.is_empty() || actions.len() != effects.len() {
        return Err(invalid());
    }
    let mut hashes = BTreeSet::new();
    let mut funding = CompletedFunding {
        cycles: 0,
        payments: 0,
    };
    for (action, effect) in actions.iter().zip(effects) {
        let hash = sha256_hex(&json::to_vec(action).map_err(|_| invalid())?);
        if effect.state != EffectState::Applied
            || effect.action_sha256 != hash
            || !hashes.insert(hash)
            || !attempts_match(action, effect)
        {
            return Err(invalid());
        }
        if let CompletedActionEvidence::Fund {
            amount,
            expected_post_cycles,
            funding_deficit_cycles,
            funding_margin_cycles,
            ledger: action_ledger,
            ..
        } = action
        {
            let paid = native_funding_applied(NativeFundingObservation {
                amount: *amount,
                expected_post_cycles: *expected_post_cycles,
                funding_deficit_cycles: *funding_deficit_cycles,
                funding_margin_cycles: *funding_margin_cycles,
                live_cycles: effect.post_cycles,
                pre_cycles: effect.pre_cycles,
            });
            if action_ledger != ledger
                || !paid
                || effect.receipt.as_ref().is_none_or(String::is_empty)
            {
                return Err(invalid());
            }
            funding.cycles = funding.cycles.checked_add(*amount).ok_or_else(invalid)?;
            funding.payments += 1;
        }
    }
    Ok(funding)
}

fn attempts_match(action: &CompletedActionEvidence, effect: &EffectRecord) -> bool {
    let mut publication_limit = 0;
    let mut maintenance_limit = 0;
    if let CompletedActionEvidence::FleetProtocol { action, .. } = action {
        match action.as_ref() {
            CompletedProtocolEvidence::PrepareStoreFixture {
                maximum_attempts, ..
            }
            | CompletedProtocolEvidence::PublishStoreFixtureChunk {
                maximum_attempts, ..
            } => {
                if *maximum_attempts == 0 {
                    return false;
                }
                publication_limit = *maximum_attempts;
            }
            CompletedProtocolEvidence::MaintainPoolReadiness {
                maximum_updates, ..
            } => maintenance_limit = *maximum_updates,
            _ => {}
        }
    }
    effect.publication_attempts <= publication_limit
        && effect.maintenance_attempts <= maintenance_limit
}

fn field<T: serde::de::DeserializeOwned>(value: &Value, name: &str) -> Result<T, EnsureStateError> {
    decode(value.get(name).cloned().ok_or_else(invalid)?)
}
fn decode<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, EnsureStateError> {
    serde_json::from_value(value).map_err(|_| invalid())
}
const fn invalid() -> EnsureStateError {
    EnsureStateError::InvalidTerminalSource
}
