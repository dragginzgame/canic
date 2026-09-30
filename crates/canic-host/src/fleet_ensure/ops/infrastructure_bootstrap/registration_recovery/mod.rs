//! Retain one explicit registration extension without replacing the bootstrap plan or receipts.

#[cfg(test)]
pub(super) mod tests;

use crate::fleet_ensure::{
    model::{
        CycleConservation, EffectState, EnsureAction, FleetEnsureCompletion,
        FleetEnsureJournalRecord, FleetEnsurePlan, FleetEnsureStateRecord,
        infrastructure_bootstrap::registration_recovery::{
            BootstrapRegistrationRecoveryRecord, BootstrapRegistrationReviewRecord,
            RECOVERY_INSPECTION_ROUNDS,
        },
    },
    ops::{
        EnsurePaths, action_sha256,
        infrastructure_bootstrap::{InfrastructureBootstrapError, registration},
        write_journal,
    },
    policy::infrastructure_bootstrap::registration_recovery::quote,
    view::infrastructure_bootstrap::InfrastructureBootstrapObservation,
};
use canic_core::cdk::utils::hash::sha256_hex;
use std::collections::BTreeMap;

const fn invalid() -> InfrastructureBootstrapError {
    InfrastructureBootstrapError::Integrity
}

/// Only a completed initialization prefix with no registration or other funding may enter recovery.
pub(in crate::fleet_ensure) fn boundary(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
) -> Result<(), InfrastructureBootstrapError> {
    let count = plan
        .canisters
        .iter()
        .map(|target| target.actions.len())
        .sum::<usize>();
    if plan.infrastructure_bootstrap.is_none()
        || journal.completion != FleetEnsureCompletion::InProgress
        || !journal.successor_phases.is_empty()
        || !journal.funding_reviews.is_empty()
        || !journal.funding_observations.is_empty()
        || journal.effects.len() != count
        || journal
            .effects
            .iter()
            .any(|effect| effect.state != EffectState::Applied)
    {
        return Err(invalid());
    }
    Ok(())
}

/// Reserve the next recovery observation before its first remote read.
pub(in crate::fleet_ensure) fn reserve(
    paths: &EnsurePaths,
    journal: &mut FleetEnsureJournalRecord,
    approval: bool,
) -> Result<(), InfrastructureBootstrapError> {
    let record = journal.bootstrap_registration_recovery.get_or_insert(
        BootstrapRegistrationRecoveryRecord {
            review_attempts: 0,
            approval_attempts: 0,
            approved: false,
            review: None,
        },
    );
    let count = if approval {
        &mut record.approval_attempts
    } else {
        &mut record.review_attempts
    };
    if *count >= RECOVERY_INSPECTION_ROUNDS {
        return Err(InfrastructureBootstrapError::InspectionBudget);
    }
    *count += 1;
    write_journal(paths, journal)?;
    Ok(())
}

fn prefix_digest(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
) -> Result<String, InfrastructureBootstrapError> {
    let count = plan
        .canisters
        .iter()
        .map(|target| target.actions.len())
        .sum::<usize>();
    let prefix = journal.effects.get(..count).ok_or_else(invalid)?;
    if prefix
        .iter()
        .any(|effect| effect.state != EffectState::Applied)
    {
        return Err(invalid());
    }
    Ok(sha256_hex(&serde_json::to_vec(prefix)?))
}

fn digest(
    review: &BootstrapRegistrationReviewRecord,
) -> Result<String, InfrastructureBootstrapError> {
    let mut body = review.clone();
    body.review_sha256.clear();
    let mut bytes = b"canic.bootstrap.registration-recovery.v1\0".to_vec();
    bytes.extend(serde_json::to_vec(&body)?);
    Ok(sha256_hex(&bytes))
}

/// Capture the canonical successor and an exact Ledger deposit per underfunded native owner.
pub(in crate::fleet_ensure) fn candidate(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    phase: &FleetEnsurePlan,
    observed: &InfrastructureBootstrapObservation,
    observed_debit: u128,
    created_at_time: u64,
) -> Result<BootstrapRegistrationReviewRecord, InfrastructureBootstrapError> {
    boundary(plan, journal)?;
    let mut review = BootstrapRegistrationReviewRecord {
        schema_version: 1,
        plan_sha256: plan.plan_sha256.clone(),
        operation_id: plan.operation_id.clone(),
        applied_effects_sha256: prefix_digest(plan, journal)?,
        created_at_time,
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
                    .ok_or_else(invalid)
            })
            .collect::<Result<_, _>>()?,
        protocol_actions: phase.protocol_actions.clone(),
        funding_actions: Vec::new(),
        observed_execution_debit_cycles: observed_debit,
        successor_burn_cycles: 0,
        recovery_burn_cycles: 0,
        maximum_execution_burn_cycles: 0,
        additional_funding_cycles: 0,
        additional_ledger_fee_cycles: 0,
        operator_cycles: observed.operator_cycles,
        review_sha256: String::new(),
    };
    populate(plan, phase, &mut review)?;
    review.review_sha256 = digest(&review)?;
    Ok(review)
}

fn populate(
    plan: &FleetEnsurePlan,
    phase: &FleetEnsurePlan,
    review: &mut BootstrapRegistrationReviewRecord,
) -> Result<(), InfrastructureBootstrapError> {
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or_else(invalid)?
        .desired();
    let source = plan.infrastructure_bootstrap.as_ref().ok_or_else(invalid)?;
    let quote = quote(plan, phase, &review.canisters)?;
    review.successor_burn_cycles = quote.successor_burn;
    review.recovery_burn_cycles = quote.recovery_burn;
    review.maximum_execution_burn_cycles = review
        .observed_execution_debit_cycles
        .checked_add(quote.successor_burn)
        .and_then(|value| value.checked_add(quote.recovery_burn))
        .ok_or_else(invalid)?
        .max(plan.conservation.maximum_execution_burn_cycles);
    review.funding_actions.clear();
    review.additional_funding_cycles = 0;
    review.additional_ledger_fee_cycles = 0;
    let latest = plan
        .canisters
        .iter()
        .flat_map(|target| &target.actions)
        .filter_map(|action| match action {
            EnsureAction::Fund {
                created_at_time, ..
            }
            | EnsureAction::Create {
                created_at_time, ..
            } => Some(*created_at_time),
            _ => None,
        })
        .max()
        .unwrap_or(plan.planned_at_time);
    if review.created_at_time <= latest {
        return Err(invalid());
    }
    for (index, (name, funding)) in quote.funding.into_iter().enumerate() {
        if funding.amount == 0 {
            continue;
        }
        let sample = &review.canisters[&name];
        review.funding_actions.push(EnsureAction::Fund {
            pool_funding: None,
            amount: funding.amount,
            created_at_time: review
                .created_at_time
                .checked_add(index as u64)
                .ok_or_else(invalid)?,
            expected_post_cycles: funding.required,
            funding_deficit_cycles: funding.deficit,
            funding_margin_cycles: funding.margin,
            ledger: desired.cycles_ledger.clone(),
            name,
            principal: sample.binding.canister_id.to_text(),
        });
        review.additional_funding_cycles = review
            .additional_funding_cycles
            .checked_add(funding.amount)
            .ok_or_else(invalid)?;
        review.additional_ledger_fee_cycles = review
            .additional_ledger_fee_cycles
            .checked_add(source.ledger_fee_cycles)
            .ok_or_else(invalid)?;
    }
    Ok(())
}

/// Persist review bytes before an approval can authorize additional effects.
pub(in crate::fleet_ensure) fn retain(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
    journal: &mut FleetEnsureJournalRecord,
    review: BootstrapRegistrationReviewRecord,
) -> Result<(), InfrastructureBootstrapError> {
    let mut candidate = journal.clone();
    let record = candidate
        .bootstrap_registration_recovery
        .as_mut()
        .ok_or_else(invalid)?;
    if record.approved || record.review.is_some() {
        return Err(invalid());
    }
    record.review = Some(review);
    verify(plan, &candidate, state)?;
    let mut content = plan.clone();
    content.protocol_actions.clone_from(
        &candidate
            .bootstrap_registration_recovery
            .as_ref()
            .and_then(|record| record.review.as_ref())
            .ok_or_else(invalid)?
            .protocol_actions,
    );
    crate::fleet_ensure::ops::plan_content::retain(paths, &content)?;
    write_journal(paths, &candidate)?;
    *journal = candidate;
    Ok(())
}

/// Journal writes retain hashes and sizes rather than re-embedding immutable publication bytes.
pub(in crate::fleet_ensure) fn journal_projection(
    journal: &FleetEnsureJournalRecord,
) -> Result<serde_json::Value, serde_json::Error> {
    let mut compact = journal.clone();
    let Some(review) = compact
        .bootstrap_registration_recovery
        .as_mut()
        .and_then(|record| record.review.take())
    else {
        return crate::fleet_ensure::json::to_value(journal);
    };
    let mut projection = crate::fleet_ensure::json::to_value(&compact)?;
    let mut review = crate::fleet_ensure::json::registration_recovery_json_value(&review)?;
    if let Some(actions) = review["protocol_actions"].as_array_mut() {
        for action in actions {
            if let Some(request) = action
                .pointer_mut("/action/request")
                .and_then(serde_json::Value::as_object_mut)
            {
                request.remove("bytes_path");
            }
        }
    }
    projection["bootstrap_registration_recovery"]["review"] = review;
    Ok(projection)
}

/// Approval changes no original plan, balance baseline or effect receipt.
pub(in crate::fleet_ensure) fn approve(
    paths: &EnsurePaths,
    journal: &mut FleetEnsureJournalRecord,
) -> Result<(), InfrastructureBootstrapError> {
    let record = journal
        .bootstrap_registration_recovery
        .as_mut()
        .ok_or_else(invalid)?;
    if record.review.is_none() || record.approval_attempts == 0 {
        return Err(invalid());
    }
    record.approved = true;
    write_journal(paths, journal)?;
    Ok(())
}

/// Verify a supplement independently of its approval bit and before using any of its budgets.
pub(in crate::fleet_ensure) fn verify(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
) -> Result<(), InfrastructureBootstrapError> {
    let Some(record) = &journal.bootstrap_registration_recovery else {
        return Ok(());
    };
    if plan.infrastructure_bootstrap.is_none()
        || record.review_attempts == 0
        || record.review_attempts > RECOVERY_INSPECTION_ROUNDS
        || record.approval_attempts > RECOVERY_INSPECTION_ROUNDS
        || !journal.funding_reviews.is_empty()
        || !journal.funding_observations.is_empty()
    {
        return Err(invalid());
    }
    let Some(review) = &record.review else {
        if record.approved || record.approval_attempts != 0 {
            return Err(invalid());
        }
        return boundary(plan, journal);
    };
    if record.approved && record.approval_attempts == 0 {
        return Err(invalid());
    }
    if !record.approved {
        boundary(plan, journal)?;
    }
    if review.schema_version != 1
        || review.plan_sha256 != plan.plan_sha256
        || review.operation_id != plan.operation_id
        || review.review_sha256 != digest(review)?
        || review.applied_effects_sha256 != prefix_digest(plan, journal)?
        || review.canisters.len() != plan.canisters.len()
        || review.protocol_actions.is_empty()
        || review.protocol_actions.len()
            > plan
                .continuation
                .as_ref()
                .ok_or_else(invalid)?
                .maximum_successor_actions as usize
    {
        return Err(invalid());
    }
    let total = review
        .canisters
        .values()
        .try_fold(0_u128, |sum, sample| {
            sum.checked_add(sample.cycles)
                .and_then(|value| value.checked_add(sample.reserved_cycles))
        })
        .ok_or_else(invalid)?;
    let expected_debit = journal
        .initial_controlled_cycles
        .checked_add(plan.conservation.maximum_new_funding_cycles)
        .ok_or_else(invalid)?
        .saturating_sub(total);
    if expected_debit != review.observed_execution_debit_cycles
        || expected_debit > plan.conservation.maximum_execution_burn_cycles
        || journal
            .initial_operator_cycles
            .checked_sub(plan.conservation.maximum_operator_debit_cycles)
            != Some(review.operator_cycles)
    {
        return Err(invalid());
    }
    let mut phase = plan.clone();
    for target in &mut phase.canisters {
        let sample = review.canisters.get(&target.name).ok_or_else(invalid)?;
        let principal = sample.binding.canister_id.to_text();
        if state
            .principals
            .get(&target.name)
            .or_else(|| state.pending_principals.get(&target.name))
            != Some(&principal)
        {
            return Err(invalid());
        }
        target.principal = Some(principal);
        target.actions.clear();
    }
    phase.protocol_actions.clone_from(&review.protocol_actions);
    let mut expected = review.clone();
    populate(plan, &phase, &mut expected)?;
    if expected != *review {
        return Err(invalid());
    }
    if let Some(successor) = journal.successor_phases.first() {
        let phase = successor.plan.as_ref().ok_or_else(invalid)?;
        if phase.protocol_actions != review.protocol_actions {
            return Err(invalid());
        }
        registration::require_budget(
            phase,
            review
                .maximum_execution_burn_cycles
                .saturating_sub(successor.execution_burn_before_phase),
        )?;
    }
    Ok(())
}

/// Additional Ledger effects occur only after explicit supplementary approval.
pub(in crate::fleet_ensure) fn funding_actions(
    journal: &FleetEnsureJournalRecord,
) -> &[EnsureAction] {
    approved(journal).map_or(&[], |review| &review.funding_actions)
}

/// Return the sole approved budget extension, if present.
pub(in crate::fleet_ensure) fn approved(
    journal: &FleetEnsureJournalRecord,
) -> Option<&BootstrapRegistrationReviewRecord> {
    journal
        .bootstrap_registration_recovery
        .as_ref()
        .filter(|record| record.approved)?
        .review
        .as_ref()
}

/// Effective accounting bounds; original retained authority is never rewritten.
pub(in crate::fleet_ensure) fn conservation(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
) -> Result<CycleConservation, InfrastructureBootstrapError> {
    let mut bounds = plan.conservation.clone();
    if let Some(review) = approved(journal) {
        bounds.maximum_execution_burn_cycles = review.maximum_execution_burn_cycles;
        bounds.maximum_new_funding_cycles = bounds
            .maximum_new_funding_cycles
            .checked_add(review.additional_funding_cycles)
            .ok_or_else(invalid)?;
        bounds.maximum_unavoidable_fee_cycles = bounds
            .maximum_unavoidable_fee_cycles
            .checked_add(review.additional_ledger_fee_cycles)
            .ok_or_else(invalid)?;
        bounds.maximum_operator_debit_cycles = bounds
            .maximum_operator_debit_cycles
            .checked_add(review.additional_funding_cycles)
            .and_then(|value| value.checked_add(review.additional_ledger_fee_cycles))
            .ok_or_else(invalid)?;
        bounds.expected_post_operation_cycles = bounds
            .observed_controlled_cycles
            .checked_add(bounds.maximum_new_funding_cycles)
            .ok_or_else(invalid)?
            .saturating_sub(bounds.maximum_execution_burn_cycles);
    }
    Ok(bounds)
}

/// New funding allowances are added once alongside the existing immutable action catalogue.
pub(in crate::fleet_ensure) fn funding_hashes(
    journal: &FleetEnsureJournalRecord,
) -> BTreeMap<String, u32> {
    funding_actions(journal)
        .iter()
        .map(|action| (action_sha256(action), 0))
        .collect()
}

/// Reuse the effect driver's existing live status read to verify each funded target.
pub(in crate::fleet_ensure) fn verify_funding_target(
    review: &BootstrapRegistrationReviewRecord,
    action: &EnsureAction,
    live: Option<&crate::fleet_ensure::model::LiveCanister>,
) -> Result<(), InfrastructureBootstrapError> {
    if !review.funding_actions.contains(action) {
        return Ok(());
    }
    let sample = review.canisters.get(action.name()).ok_or_else(invalid)?;
    let live = live.ok_or_else(invalid)?;
    let expected_controllers = sample
        .binding
        .controllers
        .iter()
        .map(candid::Principal::to_text)
        .collect::<std::collections::BTreeSet<_>>();
    let actual_controllers = live
        .controllers
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let expected = FundingTargetAuthority {
        principal: sample.binding.canister_id.to_text(),
        module_sha256: sample
            .binding
            .module_sha256
            .map(canic_core::cdk::utils::hash::hex_bytes),
        controllers: expected_controllers,
        running: true,
    };
    let current = FundingTargetAuthority {
        principal: live.principal.clone(),
        module_sha256: live.module_sha256.clone(),
        controllers: actual_controllers,
        running: live.status == crate::fleet_ensure::model::CanisterRuntimeStatus::Running,
    };
    if current != expected || current.controllers.len() != live.controllers.len() {
        return Err(invalid());
    }
    Ok(())
}

/// Exact physical authority of a supplementary Ledger destination.
#[derive(Eq, PartialEq)]
struct FundingTargetAuthority {
    principal: String,
    module_sha256: Option<String>,
    controllers: std::collections::BTreeSet<String>,
    running: bool,
}
