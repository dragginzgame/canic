//! Persist completed-source preparation without rewriting the original operation.
//!
//! Workflow owns sequencing; the existing authority-seal transport owns each seal.

pub(in crate::fleet_ensure) mod inventory;
#[cfg(test)]
mod tests;

use crate::{
    fleet_ensure::{
        CompletedEstateMembershipView, CompletedSourceInspectionView,
        model::{DesiredCanisterKind, EnsureAction, completed_handoff::preparation::*},
        ops::{self, EnsurePaths, EnsureStateError, reinstall::terminal::inventory::custody},
    },
    icp::IcpCli,
};
use candid::{CandidType, Deserialize, Nat, Principal};
use canic_core::{
    cdk::utils::hash::{hex_bytes, sha256_hex},
    dto::canister::CanisterInspectionRequest,
};
use std::path::PathBuf;
use thiserror::Error;

/// Current reviewed preparation allowance; no Ledger debit is permitted.
const ATTEMPTS: u32 = 2;
const OBSERVATIONS: u32 = 4;
const ACTION_BURN: u128 = 6_000_000_000_000;
const INSPECTION_BURN: u128 = 2_000_000_000_000;

/// A failed preparation never authorizes replacing IDs, funding or wiping state.
#[derive(Debug, Error)]
pub enum CompletedPreparationError {
    #[error(transparent)]
    State(#[from] EnsureStateError),
    #[error("completed-source preparation evidence, release or approval changed")]
    Conflict,
    #[error("completed-source preparation allowance exhausted; preserve its review and receipts")]
    Budget,
    #[error(
        "authority cannot cover its reviewed preparation burn, or its observed debit exceeded the allowance"
    )]
    Conservation,
    #[error("completed-source observation failed: {0}")]
    Observation(#[from] Box<ops::retained_contract::CompletedMembershipError>),
    #[error("completed-source evidence failed: {0}")]
    Source(#[from] Box<ops::retained_contract::RetainedContractError>),
    #[error("authority seal failed: {0}")]
    Seal(#[from] Box<ops::current_protocol::CurrentProtocolError>),
    #[error("authority management observation failed: {0}")]
    Management(#[from] Box<crate::icp::IcpManagementCallError>),
    #[error("preparation runtime unavailable: {0}")]
    Runtime(#[from] std::io::Error),
    #[error("preparation encoding failed: {0}")]
    Encode(#[from] serde_json::Error),
}

pub(in crate::fleet_ensure) fn compile(
    paths: &EnsurePaths,
    source: &CompletedSourceInspectionView,
    observed: &CompletedEstateMembershipView,
    environment: &str,
    fleet: &str,
) -> Result<CompletedPreparationReviewRecord, CompletedPreparationError> {
    let source_documents = &source.inventory.receipts.documents;
    let custody = custody::capture(observed.custody(), source_documents)
        .map_err(|_| CompletedPreparationError::Conflict)?;
    let mut actions = Vec::new();
    // Close Coordinator allocation before sealing any Root.
    for kind in [DesiredCanisterKind::Coordinator, DesiredCanisterKind::Root] {
        for (name, canister) in source
            .inventory
            .canisters
            .iter()
            .filter(|(_, c)| c.kind == kind)
        {
            let binding = source
                .source_protocols
                .get(name)
                .ok_or(CompletedPreparationError::Conflict)?;
            let path = binding
                .candid_path()
                .strip_prefix(&paths.workspace)
                .map_err(|_| CompletedPreparationError::Conflict)?;
            actions.push(EnsureAction::SealAuthority {
                authority_kind: kind,
                candid: path
                    .to_str()
                    .ok_or(CompletedPreparationError::Conflict)?
                    .into(),
                candid_sha256: hex_bytes(binding.binding().candid_sha256),
                name: name.clone(),
                principal: canister.principal.to_text(),
            });
        }
    }
    if actions.len() < 2 {
        return Err(CompletedPreparationError::Conflict);
    }
    let source_artifacts = ops::completed_handoff::retain_completed_source(paths, source)?;
    let inspection_roots = source
        .inventory
        .canisters
        .iter()
        .map(|(name, canister)| {
            let owner = match canister.kind {
                DesiredCanisterKind::Coordinator | DesiredCanisterKind::Root => None,
                _ => canister.root.clone(),
            };
            (name.clone(), owner)
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let operation_id = sha256_hex(&serde_json::to_vec(&(
        "canic:completed-preparation:v1",
        env!("CARGO_PKG_VERSION"),
        source_documents,
    ))?);
    let mut review = CompletedPreparationReviewRecord {
        schema_version: 1,
        cli_release: env!("CARGO_PKG_VERSION").into(),
        environment: environment.into(),
        fleet: fleet.into(),
        operation_id,
        source: source_documents.clone(),
        custody,
        source_artifacts,
        maximum_execution_burn_cycles: ACTION_BURN
            .checked_mul(actions.len() as u128)
            .and_then(|burn| {
                INSPECTION_BURN
                    .checked_mul(inspection_roots.len() as u128)?
                    .checked_add(burn)
            })
            .ok_or(CompletedPreparationError::Budget)?,
        actions,
        inspection_roots,
        source_accounting: CompletedSourceAccountingRecord {
            initial_native_cycles: source
                .inventory
                .receipts
                .initial_controlled_cycles
                .checked_sub(
                    source
                        .inventory
                        .receipts
                        .initial_estate_funding_cycles_by_root
                        .values()
                        .try_fold(0_u128, |sum, value| sum.checked_add(*value))
                        .ok_or(CompletedPreparationError::Conservation)?,
                )
                .ok_or(CompletedPreparationError::Conservation)?,
            recorded_funding_cycles: source.inventory.receipts.recorded_funding_cycles,
            maximum_source_burn_cycles: source
                .inventory
                .receipts
                .original_maximum_execution_burn_cycles,
        },
        maximum_attempts_per_action: ATTEMPTS,
        maximum_observations_per_action: OBSERVATIONS,
        review_sha256: String::new(),
    };
    review.review_sha256 = digest(&review)?;
    Ok(review)
}

fn digest(review: &CompletedPreparationReviewRecord) -> Result<String, serde_json::Error> {
    let mut body = review.clone();
    body.review_sha256.clear();
    Ok(sha256_hex(&serde_json::to_vec(&(
        "canic:completed-preparation-review:v1",
        body,
    ))?))
}

pub(in crate::fleet_ensure) fn review(
    paths: &EnsurePaths,
) -> Result<Option<CompletedPreparationReviewRecord>, CompletedPreparationError> {
    let value: Option<CompletedPreparationReviewRecord> = ops::read_current(&review_path(paths))?;
    if let Some(review) = &value {
        verify(review)?;
        let expected = EnsurePaths::under(&paths.workspace, &review.environment, &review.fleet);
        if expected.plan != paths.plan {
            return Err(CompletedPreparationError::Conflict);
        }
    }
    Ok(value)
}

pub(in crate::fleet_ensure) fn verify(
    review: &CompletedPreparationReviewRecord,
) -> Result<(), CompletedPreparationError> {
    crate::fleet_ensure::policy::validate_path_labels(&review.environment, &review.fleet)
        .map_err(|_| CompletedPreparationError::Conflict)?;
    let operation = sha256_hex(&serde_json::to_vec(&(
        "canic:completed-preparation:v1",
        env!("CARGO_PKG_VERSION"),
        &review.source,
    ))?);
    if review.operation_id != operation
        || review.schema_version != 1
        || review.cli_release != env!("CARGO_PKG_VERSION")
        || review.maximum_attempts_per_action != ATTEMPTS
        || review.maximum_observations_per_action != OBSERVATIONS
        || review.actions.len() < 2
        || review.maximum_execution_burn_cycles
            != ACTION_BURN
                .checked_mul(review.actions.len() as u128)
                .and_then(|burn| {
                    INSPECTION_BURN
                        .checked_mul(review.inspection_roots.len() as u128)?
                        .checked_add(burn)
                })
                .ok_or(CompletedPreparationError::Budget)?
        || digest(review)? != review.review_sha256
    {
        return Err(CompletedPreparationError::Conflict);
    }
    let mut principals = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for (index, action) in review.actions.iter().enumerate() {
        let EnsureAction::SealAuthority {
            name,
            principal,
            authority_kind,
            candid,
            candid_sha256,
        } = action
        else {
            return Err(CompletedPreparationError::Conflict);
        };
        let binding = review
            .custody
            .canisters
            .get(name)
            .ok_or(CompletedPreparationError::Conflict)?;
        let expected_kind = if index == 0 {
            DesiredCanisterKind::Coordinator
        } else {
            DesiredCanisterKind::Root
        };
        let relative = std::path::Path::new(candid);
        if binding.binding.principal.to_text() != *principal
            || *authority_kind != expected_kind
            || !names.insert(name)
            || !principals.insert(principal)
            || !ops::is_sha256(candid_sha256)
            || relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
            || !binding
                .binding
                .controllers
                .contains(&review.custody.operator)
        {
            return Err(CompletedPreparationError::Conflict);
        }
    }
    inventory::verify_review(review)
}

pub(in crate::fleet_ensure) fn stage(
    paths: &EnsurePaths,
    review: &CompletedPreparationReviewRecord,
) -> Result<(), CompletedPreparationError> {
    verify(review)?;
    if journal(paths, review)?.is_some() {
        return Err(CompletedPreparationError::Conflict);
    }
    ops::write_current(&review_path(paths), review)?;
    Ok(())
}

pub(in crate::fleet_ensure) fn journal(
    paths: &EnsurePaths,
    review: &CompletedPreparationReviewRecord,
) -> Result<Option<CompletedPreparationJournalRecord>, CompletedPreparationError> {
    let journal: Option<CompletedPreparationJournalRecord> =
        ops::read_current(&journal_path(paths))?;
    if let Some(journal) = &journal {
        if journal.schema_version != 1
            || journal.review_sha256 != review.review_sha256
            || journal.effects.len() != review.actions.len()
        {
            return Err(CompletedPreparationError::Conflict);
        }
        let mut previous_complete = true;
        for (effect, action) in journal.effects.iter().zip(&review.actions) {
            if effect.action_sha256 != ops::action_sha256(action)
                || effect.submission_attempts > ATTEMPTS
                || effect.observation_attempts > OBSERVATIONS
                || (effect.before.is_some() && effect.observation_attempts == 0)
                || (effect.applied && effect.before.is_none())
                || (!previous_complete && effect.observation_attempts != 0)
                || (effect.submission_attempts != 0 && effect.before.is_none())
                || (effect.after.is_some() && (!effect.applied || effect.before.is_none()))
                || (journal.prepared && (!effect.applied || effect.after.is_none()))
            {
                return Err(CompletedPreparationError::Conflict);
            }
            if let Some(before) = effect.before {
                if before.native_cycles < ACTION_BURN {
                    return Err(CompletedPreparationError::Conservation);
                }
                if let Some(after) = effect.after {
                    if effect.observation_attempts < 2 {
                        return Err(CompletedPreparationError::Conflict);
                    }
                    validate_debit(before, after)?;
                }
            }
            previous_complete = effect.applied && effect.after.is_some();
        }
        inventory::verify_journal(review, journal)?;
    }
    Ok(journal)
}

pub(in crate::fleet_ensure) fn begin(
    paths: &EnsurePaths,
    review: &CompletedPreparationReviewRecord,
) -> Result<CompletedPreparationJournalRecord, CompletedPreparationError> {
    let journal = CompletedPreparationJournalRecord {
        schema_version: 1,
        review_sha256: review.review_sha256.clone(),
        prepared: false,
        inspections: review
            .inspection_roots
            .keys()
            .map(|name| {
                (
                    name.clone(),
                    CompletedPreparationInspectionRecord {
                        attempts: 0,
                        balance: None,
                    },
                )
            })
            .collect(),
        effects: review
            .actions
            .iter()
            .map(|action| CompletedPreparationEffectRecord {
                action_sha256: ops::action_sha256(action),
                submission_attempts: 0,
                observation_attempts: 0,
                applied: false,
                before: None,
                after: None,
            })
            .collect(),
    };
    save(paths, &journal)?;
    Ok(journal)
}

pub(in crate::fleet_ensure) fn save(
    paths: &EnsurePaths,
    journal: &CompletedPreparationJournalRecord,
) -> Result<(), CompletedPreparationError> {
    ops::write_current(&journal_path(paths), journal)?;
    Ok(())
}

pub(in crate::fleet_ensure) fn consume(
    paths: &EnsurePaths,
    journal: &mut CompletedPreparationJournalRecord,
    index: usize,
    observation: bool,
) -> Result<(), CompletedPreparationError> {
    let effect = &mut journal.effects[index];
    let (count, limit) = if observation {
        (&mut effect.observation_attempts, OBSERVATIONS)
    } else {
        (&mut effect.submission_attempts, ATTEMPTS)
    };
    if *count >= limit {
        return Err(CompletedPreparationError::Budget);
    }
    *count += 1;
    save(paths, journal)
}

pub(in crate::fleet_ensure) fn retain_balance(
    paths: &EnsurePaths,
    journal: &mut CompletedPreparationJournalRecord,
    index: usize,
    cycles: CompletedPreparationBalanceRecord,
    after: bool,
) -> Result<(), CompletedPreparationError> {
    let effect = &mut journal.effects[index];
    if after {
        let before = effect.before.ok_or(CompletedPreparationError::Conflict)?;
        validate_debit(before, cycles)?;
        effect.after = Some(cycles);
    } else {
        if cycles.native_cycles < ACTION_BURN {
            return Err(CompletedPreparationError::Conservation);
        }
        effect.before = Some(cycles);
    }
    save(paths, journal)
}

pub(in crate::fleet_ensure) fn mark_applied(
    paths: &EnsurePaths,
    journal: &mut CompletedPreparationJournalRecord,
    index: usize,
) -> Result<(), CompletedPreparationError> {
    journal.effects[index].applied = true;
    save(paths, journal)
}
pub(in crate::fleet_ensure) fn finish(
    paths: &EnsurePaths,
    review: &CompletedPreparationReviewRecord,
    journal: &mut CompletedPreparationJournalRecord,
) -> Result<(), CompletedPreparationError> {
    if journal.effects.is_empty()
        || journal
            .effects
            .iter()
            .any(|effect| !effect.applied || effect.after.is_none())
    {
        return Err(CompletedPreparationError::Conflict);
    }
    inventory::conservation(review, journal)?;
    journal.prepared = true;
    save(paths, journal)
}

#[derive(CandidType, Deserialize)]
struct Status {
    status: canic_core::dto::canister::CanisterStatusType,
    cycles: Nat,
    reserved_cycles: Nat,
    module_hash: Option<Vec<u8>>,
    settings: Settings,
}
#[derive(CandidType, Deserialize)]
struct Settings {
    controllers: Vec<Principal>,
}

pub(in crate::fleet_ensure) fn balance(
    icp: &IcpCli,
    review: &CompletedPreparationReviewRecord,
    action: &EnsureAction,
) -> Result<CompletedPreparationBalanceRecord, CompletedPreparationError> {
    let binding = &review
        .custody
        .canisters
        .get(action.name())
        .ok_or(CompletedPreparationError::Conflict)?
        .binding;
    let status: Status = icp
        .management_canister_status_candid(
            binding.principal,
            &CanisterInspectionRequest {
                canister_id: binding.principal,
            },
        )
        .map_err(Box::new)?;
    let mut controllers = status.settings.controllers;
    controllers.sort_unstable();
    if controllers != binding.controllers
        || status.module_hash.map(hex_bytes) != binding.module_sha256
    {
        return Err(CompletedPreparationError::Conflict);
    }
    let native =
        u128::try_from(status.cycles.0).map_err(|_| CompletedPreparationError::Conservation)?;
    let reserved = u128::try_from(status.reserved_cycles.0)
        .map_err(|_| CompletedPreparationError::Conservation)?;
    Ok(CompletedPreparationBalanceRecord {
        status: runtime_status(status.status),
        native_cycles: native,
        reserved_cycles: reserved,
    })
}

pub(in crate::fleet_ensure) const fn runtime_status(
    status: canic_core::dto::canister::CanisterStatusType,
) -> crate::fleet_ensure::model::CanisterRuntimeStatus {
    use crate::fleet_ensure::model::CanisterRuntimeStatus;
    use canic_core::dto::canister::CanisterStatusType;
    match status {
        CanisterStatusType::Running => CanisterRuntimeStatus::Running,
        CanisterStatusType::Stopped => CanisterRuntimeStatus::Stopped,
        CanisterStatusType::Stopping => CanisterRuntimeStatus::Stopping,
    }
}

fn validate_debit(
    before: CompletedPreparationBalanceRecord,
    after: CompletedPreparationBalanceRecord,
) -> Result<(), CompletedPreparationError> {
    let total = |balance: CompletedPreparationBalanceRecord| {
        balance
            .native_cycles
            .checked_add(balance.reserved_cycles)
            .ok_or(CompletedPreparationError::Conservation)
    };
    let debit = total(before)?
        .checked_sub(total(after)?)
        .ok_or(CompletedPreparationError::Conservation)?;
    if debit > ACTION_BURN {
        return Err(CompletedPreparationError::Conservation);
    }
    Ok(())
}

pub(in crate::fleet_ensure) fn require_no_intent(
    paths: &EnsurePaths,
) -> Result<(), EnsureStateError> {
    if ops::read_document_bytes(&journal_path(paths))?.is_some() {
        if let Some(publication) = ops::completed_handoff::committed(paths)? {
            let preparation = review(paths)
                .map_err(|_| EnsureStateError::CompletedPreparationInProgress)?
                .ok_or(EnsureStateError::CompletedPreparationInProgress)?;
            let replacement = ops::completed_handoff::replacement_plan(paths, &publication)?;
            let exact_preparation = replacement
                .reinstall
                .as_ref()
                .and_then(|intent| intent.completed_reset.as_ref())
                .is_some_and(|completed| {
                    completed.preparation == preparation && completed.prepared.prepared
                });
            if publication.source == preparation.source
                && exact_preparation
                && ops::completed_handoff::pending(paths)?.is_none()
            {
                return Ok(());
            }
        }
        return Err(EnsureStateError::CompletedPreparationInProgress);
    }
    Ok(())
}

fn review_path(paths: &EnsurePaths) -> PathBuf {
    paths
        .plan
        .with_file_name("completed-preparation-review.json")
}
fn journal_path(paths: &EnsurePaths) -> PathBuf {
    paths
        .plan
        .with_file_name("completed-preparation-journal.json")
}

pub(in crate::fleet_ensure) fn revalidate(
    paths: &EnsurePaths,
    review: &CompletedPreparationReviewRecord,
    source: &CompletedSourceInspectionView,
    observed: &CompletedEstateMembershipView,
) -> Result<(), CompletedPreparationError> {
    let current = compile(paths, source, observed, &review.environment, &review.fleet)?;
    let stable = current.source == review.source
        && current.source_artifacts == review.source_artifacts
        && current.actions == review.actions
        && current.inspection_roots == review.inspection_roots
        && current.source_accounting == review.source_accounting
        && current.operation_id == review.operation_id
        && custody::same_authority(&current.custody, &review.custody);
    if !stable {
        return Err(CompletedPreparationError::Conflict);
    }
    Ok(())
}

pub(in crate::fleet_ensure) fn sealed(
    icp: &IcpCli,
    paths: &EnsurePaths,
    review: &CompletedPreparationReviewRecord,
    action: &EnsureAction,
) -> Result<bool, CompletedPreparationError> {
    Ok(
        ops::authority_seal::observe(icp, &paths.workspace, &review.operation_id, action)
            .map_err(Box::new)?
            .applied,
    )
}

pub(in crate::fleet_ensure) fn submit(
    icp: &IcpCli,
    paths: &EnsurePaths,
    review: &CompletedPreparationReviewRecord,
    action: &EnsureAction,
) -> Result<(), CompletedPreparationError> {
    ops::authority_seal::apply(icp, &paths.workspace, &review.operation_id, action)
        .map_err(Box::new)?;
    Ok(())
}
