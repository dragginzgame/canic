//! Construct and validate capacity-import handoff records using Fleet effect records.
//!
//! Workflow must retain each changed journal under the shared operation lock before effects.
//! These helpers never dispatch an IC request or infer success from an unavailable reply.

pub mod retirement;
mod store;
#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::{
        EffectRecord, EffectState, EnsureAction,
        capacity_import::{
            CapacityImportHandoffRecord, CapacityImportHandoffRequestRecord,
            CapacityImportJournalRecord, CapacityImportPlanRecord, CapacityImportReservationRecord,
        },
    },
    ops::{
        action_sha256,
        capacity_import::{
            CapacityImportReviewError,
            transport::{CompletedHandoff, validate_request},
            verify_review,
        },
    },
    policy::capacity_import::{
        CapacityImportPolicyError, admit_handoffs, admit_source_handoff, retained_source_debit,
        source_total,
    },
    view::capacity_import::{CapacityImportDestinationView, CapacityImportSourceView},
};
use candid::Principal;
use thiserror::Error;

pub use store::CapacityImportJournalStore;
pub(in crate::fleet_ensure) use store::require_completion_fits;
pub(in crate::fleet_ensure::ops) use store::require_no_approved_import;

/// Typed phase identifying which bounded inventory observation failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapacityImportInventoryStage {
    Decode,
    Membership,
    Page,
    Policy,
    Query,
    Summary,
}

/// Journal rejection preserves the original operation and all issued intent.
#[derive(Debug, Error)]
pub enum CapacityImportJournalError {
    #[error(
        "no original capacity observation is retained for {canister}; keep the original review inputs when declaring additional funding, or review this source before declaring a credit"
    )]
    FundingBaselineMissing { canister: Principal },
    #[error(
        "capacity import requires a completed current Fleet or receipted bootstrap setup with exact installed infrastructure records"
    )]
    InfrastructureRequired,
    #[error("capacity import infrastructure module, controllers, placement or Registry changed")]
    InfrastructureChanged,
    #[error(
        "capacity import requires an exact operator disposition declaration for every candidate"
    )]
    DispositionInvalid,
    #[error(
        "capacity import inventory is incomplete, changed, duplicated or exceeds its observation bound"
    )]
    InventoryInvalid,
    #[error("capacity import inventory observation of {canister} failed at {stage:?}")]
    InventoryObservation {
        canister: Principal,
        stage: CapacityImportInventoryStage,
    },
    #[error(
        "management observation failed for {canister}; verify operator controller access before retrying"
    )]
    ObservationUnavailable { canister: Principal },
    #[error(transparent)]
    Inspection(Box<crate::canister_protocol::CanisterProtocolError>),
    #[error(transparent)]
    Prerequisite(
        #[from] crate::fleet_ensure::ops::capacity_import::CapacityImportPrerequisiteError,
    ),
    #[error(
        "Coordinator {coordinator} is unavailable; verify its initialization, network and operator access"
    )]
    CoordinatorUnavailable { coordinator: Principal },
    #[error("Coordinator rejected the import prerequisite query: {0}")]
    CoordinatorRejected(canic_contracts::dto::error::Error),
    #[error(transparent)]
    Review(#[from] CapacityImportReviewError),
    #[error(transparent)]
    Policy(#[from] CapacityImportPolicyError),
    #[error("capacity import journal does not match the reviewed handoff sequence")]
    Integrity,
    #[error("capacity import request differs from the exact reviewed controller handoff")]
    RequestInvalid,
    #[error("capacity import transport differs from the reviewed network or signer")]
    ReaderMismatch,
    #[error("Root rejected the capacity import request: {0}")]
    RootRejected(canic_contracts::dto::error::Error),
    #[error(
        "capacity import budget cannot cover the complete operation: {paid_calls} calls, at least {minimum_calls} required; debit ceiling {maximum_debit_cycles}, required {required_debit_cycles}"
    )]
    InsufficientRootBudget {
        paid_calls: u32,
        minimum_calls: u32,
        maximum_debit_cycles: u128,
        required_debit_cycles: u128,
    },
    #[error(
        "Root capacity import hit a capacity limit in phase {phase:?}: {reserved_debit_cycles}/{maximum_debit_cycles} cycles reserved, {observed_debit_cycles} observed debit, {paid_calls}/{maximum_paid_calls} calls; preserve issued operation authority"
    )]
    RootCapacityLimit {
        phase: canic_contracts::dto::pool_import::PoolImportPhase,
        reserved_debit_cycles: u128,
        maximum_debit_cycles: u128,
        observed_debit_cycles: u128,
        paid_calls: u32,
        maximum_paid_calls: u32,
    },
    #[error("Root capacity import response has an invalid current-contract encoding")]
    RootResponseInvalid,
    #[error("capacity import requires exact approval and a protected Root reservation")]
    ReservationRequired,
    #[error("capacity import effect outcome is unresolved; retain its original intent")]
    Unresolved,

    #[error(
        "capacity import exhausted its reviewed {step} attempts; preserve the original journal and review continuation with canic --environment <environment> fleet recover-attempts <fleet>"
    )]
    BudgetExhausted { step: String },
    #[error("capacity import inventory inputs or publication evidence changed")]
    PublicationConflict,
    #[error("capacity import generator inputs are invalid: {0}")]
    InventoryProjection(
        Box<crate::fleet_ensure::generate::capacity_import::CapacityImportInventoryError>,
    ),
    #[error("another capacity import already retains approval or effect intent")]
    Conflict,
    #[error(transparent)]
    State(#[from] crate::fleet_ensure::ops::EnsureStateError),
    #[error("capacity import journal file operation failed")]
    Io(#[from] std::io::Error),
    #[error("capacity import journal cannot be decoded or encoded")]
    Encoding(#[from] serde_json::Error),
}

impl From<crate::canister_protocol::CanisterProtocolError> for CapacityImportJournalError {
    fn from(error: crate::canister_protocol::CanisterProtocolError) -> Self {
        Self::Inspection(Box::new(error))
    }
}

/// Construct the initial journal without approval or effects.
pub fn reviewed(
    plan: CapacityImportPlanRecord,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    verify_review(&plan, plan.plan_sha256)?;
    Ok(CapacityImportJournalRecord {
        handoffs: plan
            .sources
            .iter()
            .map(|source| CapacityImportHandoffRecord {
                canister_id: source.binding.canister_id,
                retirements: Vec::new(),
                effect: None,
                request: None,
                before_reserved_cycles: None,
                after_reserved_cycles: None,
            })
            .collect(),
        plan,
        operation: None,
        approved: false,
        reservation: None,
    })
}

/// Bind explicit approval to fresh, complete authority and obligation observations.
pub fn approve(
    journal: &CapacityImportJournalRecord,
    digest: [u8; 32],
    destination: &CapacityImportDestinationView,
    sources: &[CapacityImportSourceView],
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    validate(journal)?;
    verify_review(&journal.plan, digest)?;
    if journal.approved {
        return Ok(journal.clone());
    }
    admit_handoffs(&journal.plan, destination, sources)?;
    let mut updated = journal.clone();
    updated.approved = true;
    Ok(updated)
}

/// Retain a reservation only after the transport authenticates the Root's protected reply.
pub fn reserve(
    journal: &CapacityImportJournalRecord,
    reservation: CapacityImportReservationRecord,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    validate(journal)?;
    if !journal.approved || !reservation_matches(&journal.plan, &reservation) {
        return Err(CapacityImportJournalError::ReservationRequired);
    }
    if let Some(retained) = &journal.reservation {
        if *retained != reservation {
            return Err(CapacityImportJournalError::Integrity);
        }
        return Ok(journal.clone());
    }
    let mut updated = journal.clone();
    updated.reservation = Some(reservation);
    Ok(updated)
}

/// Retain one exact intent before issuing the controller update.
/// A repeated call returns the existing intent; it grants no additional issuance.
pub fn prepare_handoff(
    journal: &CapacityImportJournalRecord,
    observed: &CapacityImportSourceView,
    request: CapacityImportHandoffRequestRecord,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    validate(journal)?;
    if !journal.approved || journal.reservation.is_none() {
        return Err(CapacityImportJournalError::ReservationRequired);
    }
    let index = source_index(journal, observed.binding.canister_id)?;
    if journal.handoffs[index].effect.is_some() {
        return Ok(journal.clone());
    }
    let source = &journal.plan.sources[index];
    if !crate::fleet_ensure::policy::capacity_import::requires_handoff(&journal.plan, source) {
        return Err(CapacityImportJournalError::Integrity);
    }
    admit_source_handoff(source, observed)?;
    validate_request(&journal.plan, observed.binding.canister_id, &request)?;
    let mut updated = journal.clone();
    updated.handoffs[index].effect = Some(handoff_intent(&journal.plan, index, observed.cycles));
    updated.handoffs[index].before_reserved_cycles = Some(observed.reserved_cycles);
    updated.handoffs[index].request = Some(request);
    Ok(updated)
}

fn handoff_intent(
    plan: &CapacityImportPlanRecord,
    index: usize,
    observed_cycles: u128,
) -> EffectRecord {
    EffectRecord {
        maintenance_attempts: 0,
        publication_attempts: 0,
        action_sha256: action_sha256(&handoff_action(plan, index)),
        created_principal: None,
        destination_post_cycles: None,
        destination_pre_cycles: None,
        post_cycles: None,
        pre_cycles: Some(observed_cycles),
        pre_canister_version: Some(plan.sources[index].binding.canister_version),
        progress_identity: None,
        receipt: None,
        state: EffectState::Intent,
    }
}

/// Record issuance before submitting the exact action. Issued intent cannot be reissued here.
pub fn issue_handoff(
    journal: &CapacityImportJournalRecord,
    canister: Principal,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    validate(journal)?;
    let index = source_index(journal, canister)?;
    let mut updated = journal.clone();
    let effect = updated.handoffs[index]
        .effect
        .as_mut()
        .ok_or(CapacityImportJournalError::Integrity)?;
    if effect.state != EffectState::Intent {
        return Err(CapacityImportJournalError::Unresolved);
    }
    effect.state = EffectState::Issued;
    Ok(updated)
}

/// Reconcile a lost update reply only against the exact post-handoff management identity.
/// Original controllers, failed reads and foreign changes never establish non-execution.
pub fn observe_handoff(
    journal: &CapacityImportJournalRecord,
    observed: &CapacityImportSourceView,
    completion: &CompletedHandoff,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    validate(journal)?;
    let index = source_index(journal, observed.binding.canister_id)?;
    let source = &journal.plan.sources[index];
    let request = journal.handoffs[index]
        .request
        .as_ref()
        .ok_or(CapacityImportJournalError::Integrity)?;
    if !completion.matches(&journal.plan, source.binding.canister_id, request) {
        return Err(CapacityImportJournalError::Unresolved);
    }
    apply_observed_handoff(journal, observed)
}

// Called only after exact certified reply or retained terminal ingress evidence.
fn apply_observed_handoff(
    journal: &CapacityImportJournalRecord,
    observed: &CapacityImportSourceView,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    validate(journal)?;
    let index = source_index(journal, observed.binding.canister_id)?;
    let effect = journal.handoffs[index]
        .effect
        .as_ref()
        .ok_or(CapacityImportJournalError::Integrity)?;
    if effect.state == EffectState::Applied {
        return Ok(journal.clone());
    }
    if effect.state != EffectState::Issued {
        return Err(CapacityImportJournalError::Integrity);
    }
    let source = &journal.plan.sources[index];
    let mut expected = source.binding.clone();
    expected
        .controllers
        .clone_from(&journal.plan.transitional_controllers);
    expected.canister_version = expected
        .canister_version
        .checked_add(1)
        .ok_or(CapacityImportJournalError::Integrity)?;
    if observed.binding != expected {
        return Err(CapacityImportJournalError::Unresolved);
    }
    retained_source_debit(source, observed.cycles, observed.reserved_cycles)?;
    let before = source_total(
        effect
            .pre_cycles
            .ok_or(CapacityImportJournalError::Integrity)?,
        journal.handoffs[index]
            .before_reserved_cycles
            .ok_or(CapacityImportJournalError::Integrity)?,
    )?;
    if before.saturating_sub(source_total(observed.cycles, observed.reserved_cycles)?)
        > source.maximum_debit_cycles
    {
        return Err(CapacityImportJournalError::Unresolved);
    }
    let mut updated = journal.clone();
    let effect = updated.handoffs[index]
        .effect
        .as_mut()
        .ok_or(CapacityImportJournalError::Integrity)?;
    effect.state = EffectState::Applied;
    effect.post_cycles = Some(observed.cycles);
    // The receipt records the observed version, not a synthetic IC request ID.
    effect.receipt = Some(expected.canister_version.to_string());
    updated.handoffs[index].after_reserved_cycles = Some(observed.reserved_cycles);
    validate(&updated)?;
    Ok(updated)
}

/// Reject malformed or substituted records before any workflow resumes them.
pub fn validate(journal: &CapacityImportJournalRecord) -> Result<(), CapacityImportJournalError> {
    verify_review(&journal.plan, journal.plan.plan_sha256)?;
    crate::fleet_ensure::ops::capacity_import::publication::validate(journal)?;
    if journal.handoffs.len() != journal.plan.sources.len() {
        return Err(CapacityImportJournalError::Integrity);
    }
    if let Some(reservation) = &journal.reservation
        && (!journal.approved || !reservation_matches(&journal.plan, reservation))
    {
        return Err(CapacityImportJournalError::Integrity);
    }
    for (index, handoff) in journal.handoffs.iter().enumerate() {
        let source = &journal.plan.sources[index];
        retirement::validate(
            &journal.plan,
            handoff,
            retirement::maximum_requests(journal, handoff.canister_id)?,
        )?;
        if handoff.canister_id != source.binding.canister_id {
            return Err(CapacityImportJournalError::Integrity);
        }
        let Some(effect) = &handoff.effect else {
            if !handoff.retirements.is_empty()
                || handoff.request.is_some()
                || handoff.before_reserved_cycles.is_some()
                || handoff.after_reserved_cycles.is_some()
            {
                return Err(CapacityImportJournalError::Integrity);
            }
            continue;
        };
        if !crate::fleet_ensure::policy::capacity_import::requires_handoff(&journal.plan, source) {
            return Err(CapacityImportJournalError::Integrity);
        }
        validate_request(
            &journal.plan,
            handoff.canister_id,
            handoff
                .request
                .as_ref()
                .ok_or(CapacityImportJournalError::Integrity)?,
        )?;
        let action_bound =
            effect.action_sha256 == action_sha256(&handoff_action(&journal.plan, index));
        let authority_bound = journal.approved
            && journal.reservation.is_some()
            && effect.pre_canister_version == Some(source.binding.canister_version);
        let counters_empty = effect.maintenance_attempts == 0 && effect.publication_attempts == 0;
        let unrelated_empty = effect.created_principal.is_none()
            && effect.destination_pre_cycles.is_none()
            && effect.destination_post_cycles.is_none()
            && effect.progress_identity.is_none();
        if !action_bound || !authority_bound || !counters_empty || !unrelated_empty {
            return Err(CapacityImportJournalError::Integrity);
        }
        let before = effect
            .pre_cycles
            .ok_or(CapacityImportJournalError::Integrity)?;
        let before_reserved = handoff
            .before_reserved_cycles
            .ok_or(CapacityImportJournalError::Integrity)?;
        retained_source_debit(source, before, before_reserved)?;
        match effect.state {
            EffectState::Intent | EffectState::Issued => {
                if effect.post_cycles.is_some()
                    || effect.receipt.is_some()
                    || handoff.after_reserved_cycles.is_some()
                {
                    return Err(CapacityImportJournalError::Integrity);
                }
            }
            EffectState::Applied => {
                let after = effect
                    .post_cycles
                    .ok_or(CapacityImportJournalError::Integrity)?;
                let version = source
                    .binding
                    .canister_version
                    .checked_add(1)
                    .ok_or(CapacityImportJournalError::Integrity)?;
                let after_reserved = handoff
                    .after_reserved_cycles
                    .ok_or(CapacityImportJournalError::Integrity)?;
                let interval_debit = source_total(before, before_reserved)?
                    .saturating_sub(source_total(after, after_reserved)?);
                let receipt_matches =
                    effect.receipt.as_deref() == Some(version.to_string().as_str());
                if interval_debit > source.maximum_debit_cycles || !receipt_matches {
                    return Err(CapacityImportJournalError::Integrity);
                }
                retained_source_debit(source, after, after_reserved)?;
            }
        }
    }
    Ok(())
}

/// A source already held by Root requires no host controller effect.
/// All other sources require a certified completion retained by this journal owner.
#[must_use]
pub fn custody_ready(journal: &CapacityImportJournalRecord, canister: Principal) -> bool {
    let Some(index) = journal
        .plan
        .sources
        .iter()
        .position(|source| source.binding.canister_id == canister)
    else {
        return false;
    };
    let Some(handoff) = journal.handoffs.get(index) else {
        return false;
    };
    if handoff.canister_id != canister {
        return false;
    }
    if crate::fleet_ensure::policy::capacity_import::requires_handoff(
        &journal.plan,
        &journal.plan.sources[index],
    ) {
        handoff
            .effect
            .as_ref()
            .is_some_and(|effect| effect.state == EffectState::Applied)
    } else {
        handoff.effect.is_none()
            && handoff.request.is_none()
            && handoff.retirements.is_empty()
            && handoff.before_reserved_cycles.is_none()
            && handoff.after_reserved_cycles.is_none()
    }
}

/// Whether every selected source has retained host completion or reviewed Root custody.
#[must_use]
pub fn all_custody_ready(journal: &CapacityImportJournalRecord) -> bool {
    journal.handoffs.len() == journal.plan.sources.len()
        && journal
            .plan
            .sources
            .iter()
            .all(|source| custody_ready(journal, source.binding.canister_id))
}

fn source_index(
    journal: &CapacityImportJournalRecord,
    canister: Principal,
) -> Result<usize, CapacityImportJournalError> {
    journal
        .plan
        .sources
        .iter()
        .position(|source| source.binding.canister_id == canister)
        .ok_or(CapacityImportJournalError::Integrity)
}

fn handoff_action(plan: &CapacityImportPlanRecord, index: usize) -> EnsureAction {
    let canister = plan.sources[index].binding.canister_id.to_text();
    EnsureAction::SetControllers {
        controller_canisters: Vec::new(),
        controllers: plan
            .transitional_controllers
            .iter()
            .map(Principal::to_text)
            .collect(),
        name: canister.clone(),
        principal: canister,
    }
}

fn reservation_matches(
    plan: &CapacityImportPlanRecord,
    reservation: &CapacityImportReservationRecord,
) -> bool {
    reservation.plan_sha256 == plan.plan_sha256
        && reservation.authority == plan.authority
        && reservation.sources
            == plan
                .sources
                .iter()
                .map(|source| source.binding.canister_id)
                .collect::<Vec<_>>()
}
