//! Retain a completed infrastructure phase before publishing its journal completion.

use crate::fleet_ensure::{
    model::{
        ActualCycleConservation, EffectState, EnsureAction, FleetEnsureCompletion,
        FleetEnsureJournalRecord, FleetEnsurePlan, FleetEnsureStateRecord,
        infrastructure_bootstrap::InfrastructureBootstrapTerminalRecord,
    },
    ops::{EnsurePaths, infrastructure_bootstrap::InfrastructureBootstrapError},
    view::infrastructure_bootstrap::InfrastructureBootstrapObservation,
};

use serde::Serialize;
use sha2_host::{Digest, Sha256};
use std::path::PathBuf;

use ic_host_fs::durable::{read_optional_regular_bytes_bounded, write_bytes};

const MAXIMUM_BYTES: usize = 4 * 1024 * 1024;

pub(super) fn path(paths: &EnsurePaths, plan: &FleetEnsurePlan) -> PathBuf {
    paths
        .plan
        .with_file_name("infrastructure-bootstrap-receipts")
        .join(format!("{}.json", plan.plan_sha256))
}

fn digest(value: &impl Serialize) -> Result<[u8; 32], InfrastructureBootstrapError> {
    Ok(Sha256::digest(serde_json::to_vec(value)?).into())
}

fn journal_digest(
    journal: &FleetEnsureJournalRecord,
) -> Result<[u8; 32], InfrastructureBootstrapError> {
    let mut completed = journal.clone();
    completed.completion = FleetEnsureCompletion::Converged;
    completed.stalled_observations = 0;
    digest(&completed)
}

/// Retain exact terminal evidence; a restart may finish the remaining local journal write.
pub(in crate::fleet_ensure) fn retain(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    actual: ActualCycleConservation,
    observed: &InfrastructureBootstrapObservation,
) -> Result<(), InfrastructureBootstrapError> {
    let receipt = InfrastructureBootstrapTerminalRecord {
        schema_version: 1,
        plan_sha256: plan.plan_sha256.clone(),
        journal_sha256: journal_digest(journal)?,
        state_sha256: digest(state)?,
        actual,
        canisters: plan
            .canisters
            .iter()
            .map(|target| {
                observed
                    .canisters
                    .get(&target.name)
                    .and_then(Option::as_ref)
                    .cloned()
                    .map(|sample| (target.name.clone(), sample))
                    .ok_or(InfrastructureBootstrapError::Integrity)
            })
            .collect::<Result<_, _>>()?,
    };
    let bytes = serde_json::to_vec_pretty(&receipt)?;
    super::registration_recovery::verify(plan, journal, state)?;
    verify_effects(plan, journal)?;
    verify_registration(journal, state)?;
    verify_accounting(plan, journal, &receipt.actual)?;
    verify_membership(plan, state, &receipt)?;
    if bytes.len() > MAXIMUM_BYTES {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    let destination = path(paths, plan);
    if let Some(original) = read_optional_regular_bytes_bounded(&destination, MAXIMUM_BYTES)
        .map_err(|_| InfrastructureBootstrapError::Integrity)?
    {
        if original != bytes {
            return Err(InfrastructureBootstrapError::Integrity);
        }
        return Ok(());
    }
    write_bytes(&destination, &bytes)?;
    Ok(())
}

/// Replay locally before resolving the signer or contacting an initialized canister.
pub(in crate::fleet_ensure) fn read(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
) -> Result<Option<ActualCycleConservation>, InfrastructureBootstrapError> {
    Ok(read_receipt(paths, plan, journal, state)?.map(|receipt| receipt.actual))
}

/// Return complete validated custody for the following Root import's original cycle baseline.
pub(in crate::fleet_ensure) fn read_receipt(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
) -> Result<Option<InfrastructureBootstrapTerminalRecord>, InfrastructureBootstrapError> {
    if plan.infrastructure_bootstrap.is_none() {
        return Ok(None);
    }
    let Some(bytes) = read_optional_regular_bytes_bounded(&path(paths, plan), MAXIMUM_BYTES)
        .map_err(|_| InfrastructureBootstrapError::Integrity)?
    else {
        return Ok(None);
    };
    let receipt: InfrastructureBootstrapTerminalRecord = serde_json::from_slice(&bytes)?;
    super::registration_recovery::verify(plan, journal, state)?;
    let bounds = super::registration_recovery::conservation(plan, journal)?;
    if receipt.schema_version != 1
        || receipt.plan_sha256 != plan.plan_sha256
        || receipt.journal_sha256 != journal_digest(journal)?
        || receipt.state_sha256 != digest(state)?
        || receipt.canisters.len() != plan.canisters.len()
        || receipt.actual.observed_starting_cycles != plan.conservation.observed_controlled_cycles
        || receipt.actual.observed_net_cycle_debit_cycles > bounds.maximum_execution_burn_cycles
        || receipt.actual.operator_debit_cycles > bounds.maximum_operator_debit_cycles
    {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    let total = receipt
        .canisters
        .values()
        .try_fold(0_u128, |sum, sample| {
            sum.checked_add(sample.cycles)
                .and_then(|n| n.checked_add(sample.reserved_cycles))
        })
        .ok_or(InfrastructureBootstrapError::Integrity)?;
    if total != receipt.actual.final_controlled_cycles {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    verify_effects(plan, journal)?;
    verify_registration(journal, state)?;
    verify_accounting(plan, journal, &receipt.actual)?;
    verify_membership(plan, state, &receipt)?;
    Ok(Some(receipt))
}

fn verify_effects(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
) -> Result<(), InfrastructureBootstrapError> {
    let actions = plan
        .canisters
        .iter()
        .flat_map(|canister| &canister.actions)
        .chain(super::registration_recovery::funding_actions(journal))
        .chain(
            journal
                .successor_phases
                .iter()
                .filter_map(|phase| phase.plan.as_ref())
                .flat_map(|phase| &phase.protocol_actions),
        )
        .collect::<Vec<_>>();
    let shape_is_exact = matches!(
        journal.completion,
        FleetEnsureCompletion::InProgress | FleetEnsureCompletion::Converged
    ) && journal.effects.len() == actions.len()
        && journal.successor_phases.len() == 1
        && journal.funding_reviews.is_empty()
        && journal.funding_observations.is_empty()
        && journal.initial_controlled_cycles == plan.conservation.observed_controlled_cycles;
    let effects_are_applied = actions
        .iter()
        .zip(&journal.effects)
        .all(|(action, effect)| {
            effect.state == EffectState::Applied
                && effect.action_sha256 == crate::fleet_ensure::ops::action_sha256(action)
                && (!matches!(
                    action,
                    EnsureAction::Create { .. } | EnsureAction::Fund { .. }
                ) || effect.receipt.is_some())
        });
    if !shape_is_exact || !effects_are_applied {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    Ok(())
}

fn verify_accounting(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    actual: &ActualCycleConservation,
) -> Result<(), InfrastructureBootstrapError> {
    let reviewed = super::registration_recovery::conservation(plan, journal)?;
    let funding_is_exact = actual.estate_funding_cycles == 0
        && actual.exact_estate_creation_fee_cycles == 0
        && actual.exact_unavoidable_fee_cycles == reviewed.maximum_unavoidable_fee_cycles
        && actual.received_new_funding_cycles == reviewed.maximum_new_funding_cycles
        && actual.operator_debit_cycles == reviewed.maximum_operator_debit_cycles;
    let available = actual
        .observed_starting_cycles
        .checked_add(actual.received_new_funding_cycles)
        .ok_or(InfrastructureBootstrapError::Integrity)?;
    let debit_is_exact = actual.observed_net_cycle_debit_cycles
        == available.saturating_sub(actual.final_controlled_cycles)
        && actual.observed_net_cycle_credit_cycles
            == actual.final_controlled_cycles.saturating_sub(available);
    if !funding_is_exact || !debit_is_exact {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    Ok(())
}

fn verify_membership(
    plan: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
    receipt: &InfrastructureBootstrapTerminalRecord,
) -> Result<(), InfrastructureBootstrapError> {
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(InfrastructureBootstrapError::Integrity)?
        .desired();
    for target in &plan.canisters {
        let sample = receipt
            .canisters
            .get(&target.name)
            .ok_or(InfrastructureBootstrapError::Integrity)?;
        let configured = desired
            .canisters
            .iter()
            .find(|configured| configured.name == target.name)
            .ok_or(InfrastructureBootstrapError::Integrity)?;
        let expected = configured
            .principal
            .as_ref()
            .or_else(|| state.principals.get(&target.name))
            .ok_or(InfrastructureBootstrapError::Integrity)?;
        let module = target
            .actions
            .iter()
            .find_map(|action| match action {
                EnsureAction::Install { wasm_sha256, .. } => Some(wasm_sha256.clone()),
                _ => None,
            })
            .or_else(|| {
                plan.infrastructure_bootstrap
                    .as_ref()?
                    .sources
                    .get(&target.name)?
                    .sample
                    .binding
                    .module_sha256
                    .map(canic_core::cdk::utils::hash::hex_bytes)
            });
        let mut controllers = configured
            .controllers
            .iter()
            .map(|id| {
                candid::Principal::from_text(id)
                    .map_err(|_| InfrastructureBootstrapError::Integrity)
            })
            .collect::<Result<Vec<_>, _>>()?;
        for name in &configured.controller_canisters {
            let id = state
                .principals
                .get(name)
                .ok_or(InfrastructureBootstrapError::Integrity)?;
            controllers.push(
                candid::Principal::from_text(id)
                    .map_err(|_| InfrastructureBootstrapError::Integrity)?,
            );
        }
        controllers.sort_unstable();
        controllers.dedup();
        let minimum = configured
            .minimum_cycles
            .parse::<canic_core::cdk::types::Cycles>()
            .map_err(|_| InfrastructureBootstrapError::Integrity)?
            .to_u128();
        if sample.binding.canister_id.to_text() != *expected
            || sample.binding.subnet.to_string() != configured.subnet
            || sample
                .binding
                .module_sha256
                .map(canic_core::cdk::utils::hash::hex_bytes)
                != module
            || sample.binding.controllers != controllers
            || sample.cycles < minimum
            || sample.binding.stopped
            || sample.binding.snapshots_size_bytes != 0
        {
            return Err(InfrastructureBootstrapError::Integrity);
        }
    }
    Ok(())
}

fn verify_registration(
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
) -> Result<(), InfrastructureBootstrapError> {
    let phase = journal
        .successor_phases
        .first()
        .and_then(|phase| phase.plan.as_ref())
        .ok_or(InfrastructureBootstrapError::Integrity)?;
    let expected = super::registration::registry(phase)?;
    if state.active_registry.as_ref() != Some(&expected) {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    Ok(())
}
