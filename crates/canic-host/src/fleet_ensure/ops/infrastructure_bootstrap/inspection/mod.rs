//! Reserve finite inspection rounds before issuing management observations.
//!
//! Callers hold the existing Fleet operation lock; interrupted calls remain spent.

use crate::{
    durable_io::{read_optional_regular_bytes_bounded, write_bytes},
    fleet_ensure::{
        model::{
            EnsureAction, FleetEnsurePlan,
            infrastructure_bootstrap::{
                BOOTSTRAP_EFFECT_INSPECTION_ROUNDS, BOOTSTRAP_PHASE_INSPECTION_ROUNDS,
                InfrastructureBootstrapInspectionRecord,
            },
        },
        ops::{EnsurePaths, infrastructure_bootstrap::InfrastructureBootstrapError},
    },
};
use canic_core::cdk::utils::hash::hex_bytes;

/// Inspection boundary whose complete batch is reserved before its first paid read.
#[derive(Clone, Copy, Debug)]
pub(in crate::fleet_ensure) enum InspectionPhase {
    Review,
    Apply,
    Terminal,
    Registration,
    Effect,
}

/// A second round permits recovery of one lost response without resetting the original baseline.
pub(in crate::fleet_ensure) fn reserve(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    phase: InspectionPhase,
) -> Result<(), InfrastructureBootstrapError> {
    reserve_inner(paths, plan, phase, None)
}

/// Reserve preparation/reconciliation before the ordinary effect driver reads live state.
pub(in crate::fleet_ensure) fn reserve_effect(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    action: &EnsureAction,
) -> Result<(), InfrastructureBootstrapError> {
    if plan.infrastructure_bootstrap.is_none() {
        return Ok(());
    }
    let hash = crate::fleet_ensure::ops::action_sha256(action);
    reserve_inner(paths, plan, InspectionPhase::Effect, Some(&hash))
}

fn reserve_inner(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    phase: InspectionPhase,
    action: Option<&str>,
) -> Result<(), InfrastructureBootstrapError> {
    let source = plan
        .infrastructure_bootstrap
        .as_ref()
        .ok_or(InfrastructureBootstrapError::Integrity)?;
    let path = paths
        .plan
        .with_file_name("infrastructure-bootstrap-inspections")
        .join(format!("{}.json", hex_bytes(source.source_sha256)));
    let retained = read_optional_regular_bytes_bounded(&path, 256 * 1024)
        .map_err(|_| InfrastructureBootstrapError::Integrity)?;
    let mut record: InfrastructureBootstrapInspectionRecord = match retained {
        Some(bytes) => serde_json::from_slice(&bytes)?,
        None => InfrastructureBootstrapInspectionRecord {
            schema_version: 1,
            plan_sha256: plan.plan_sha256.clone(),
            review_attempts: 0,
            apply_attempts: 0,
            terminal_attempts: 0,
            registration_attempts: 0,
            registration_plan_sha256: None,
            effect_observations: plan
                .canisters
                .iter()
                .flat_map(|canister| &canister.actions)
                .map(|action| (crate::fleet_ensure::ops::action_sha256(action), 0))
                .collect(),
        },
    };
    if record.schema_version != 1
        || record.plan_sha256 != plan.plan_sha256
        || [
            record.review_attempts,
            record.apply_attempts,
            record.terminal_attempts,
            record.registration_attempts,
        ]
        .iter()
        .any(|n| *n > BOOTSTRAP_PHASE_INSPECTION_ROUNDS)
    {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    let expected = plan
        .canisters
        .iter()
        .flat_map(|canister| &canister.actions)
        .map(crate::fleet_ensure::ops::action_sha256)
        .collect::<std::collections::BTreeSet<_>>();
    let mut expected = expected;
    bind_registration_allowances(paths, plan, &mut record, &mut expected)?;
    if record
        .effect_observations
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        != expected
        || record
            .effect_observations
            .values()
            .any(|count| *count > BOOTSTRAP_EFFECT_INSPECTION_ROUNDS)
    {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    let counter = match phase {
        InspectionPhase::Review => &mut record.review_attempts,
        InspectionPhase::Apply => &mut record.apply_attempts,
        InspectionPhase::Terminal => &mut record.terminal_attempts,
        InspectionPhase::Registration => &mut record.registration_attempts,
        InspectionPhase::Effect => record
            .effect_observations
            .get_mut(action.ok_or(InfrastructureBootstrapError::Integrity)?)
            .ok_or(InfrastructureBootstrapError::Integrity)?,
    };
    let maximum = if matches!(phase, InspectionPhase::Effect) {
        BOOTSTRAP_EFFECT_INSPECTION_ROUNDS
    } else {
        BOOTSTRAP_PHASE_INSPECTION_ROUNDS
    };
    if *counter >= maximum {
        return Err(InfrastructureBootstrapError::InspectionBudget);
    }
    *counter += 1;
    write_bytes(&path, &serde_json::to_vec(&record)?)?;
    Ok(())
}

/// Extend the allowance map once for the immutable successor; never reset a retained counter.
fn bind_registration_allowances(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    record: &mut InfrastructureBootstrapInspectionRecord,
    expected: &mut std::collections::BTreeSet<String>,
) -> Result<(), InfrastructureBootstrapError> {
    let journal = crate::fleet_ensure::ops::read_journal(paths)?;
    let successor = journal
        .as_ref()
        .filter(|journal| journal.plan_sha256 == plan.plan_sha256)
        .and_then(|journal| journal.successor_phases.first());
    if let Some(phase) = successor {
        let actions = &phase
            .plan
            .as_ref()
            .ok_or(InfrastructureBootstrapError::Integrity)?
            .protocol_actions;
        if record.registration_plan_sha256.is_none() {
            if record
                .effect_observations
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>()
                != *expected
            {
                return Err(InfrastructureBootstrapError::Integrity);
            }
            for action in actions {
                record
                    .effect_observations
                    .insert(crate::fleet_ensure::ops::action_sha256(action), 0);
            }
            record.registration_plan_sha256 = Some(phase.plan_sha256.clone());
        }
        if record.registration_plan_sha256.as_ref() != Some(&phase.plan_sha256) {
            return Err(InfrastructureBootstrapError::Integrity);
        }
        expected.extend(actions.iter().map(crate::fleet_ensure::ops::action_sha256));
    } else if record.registration_plan_sha256.is_some() {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    Ok(())
}
