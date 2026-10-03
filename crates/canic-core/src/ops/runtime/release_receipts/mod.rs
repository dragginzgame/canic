//! Module: ops::runtime::release_receipts
//!
//! Responsibility: project bounded retained replay authority for release discovery.
//! Does not own: endpoint authentication, resumption, pruning or settlement decisions.
//! Boundary: validate exact slot identity and retain expired uncertainty without mutation.

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    dto::release_receipts::{
        ReplayReleaseAuthentication as Authentication, ReplayReleaseEffect as Effect,
        ReplayReleaseEntry, ReplayReleaseIntent, ReplayReleaseIntentState,
        ReplayReleasePhase as Phase, ReplayReleaseRecoveryReason as Reason, ReplayReleaseResponse,
        ReplayReleaseSettlement,
    },
    ids::IntentId,
    model::replay::{
        AuthKind, CommandKind, ExternalEffectDescriptor, OperationId,
        REPLAY_RECEIPT_SCHEMA_VERSION, RecoveryReason, ReplayReceiptStatus,
    },
    ops::storage::{intent::IntentStoreOps, replay::ReplayReceiptOps},
    storage::stable::{
        intent::{IntentRecord, IntentState},
        replay::ReplayReceiptEntryRecord,
    },
};
use candid::Principal;

const MAXIMUM_IDENTITY_BYTES: usize = 1024;

/// Read one stable receipt while leaving every original effect and accounting owner intact.
pub fn observe(
    owner: Principal,
    start_after: Option<[u8; 32]>,
) -> Result<ReplayReleaseResponse, InternalError> {
    let page = ReplayReceiptOps::release_page(start_after);
    let entry = page.entry.map(project).transpose()?;
    let next_after = entry
        .as_ref()
        .filter(|_| page.has_more)
        .map(|entry| entry.slot);
    Ok(ReplayReleaseResponse {
        owner,
        entry,
        next_after,
    })
}

fn project(entry: ReplayReceiptEntryRecord) -> Result<ReplayReleaseEntry, InternalError> {
    let record = entry.record;
    if record.schema_version != REPLAY_RECEIPT_SCHEMA_VERSION {
        return Err(InternalError::invariant());
    }
    bounded_identity(&record.command_kind)?;
    let command =
        CommandKind::new(record.command_kind.clone()).map_err(|_| InternalError::invariant())?;
    if ReplayReceiptOps::slot_key(&command, OperationId::from_bytes(record.operation_id))
        != entry.key
    {
        return Err(InternalError::conflict());
    }
    let authentication = match record.actor.auth_kind {
        AuthKind::DirectCaller => Authentication::DirectCaller,
        AuthKind::DelegatedToken => Authentication::DelegatedToken,
        AuthKind::RoleAttestation => Authentication::RoleAttestation,
    };
    let phase = match record.status {
        ReplayReceiptStatus::Reserved => Phase::Reserved,
        ReplayReceiptStatus::ExternalEffectInFlight => Phase::ExternalEffectInFlight,
        ReplayReceiptStatus::Committed => Phase::Committed,
        ReplayReceiptStatus::RecoveryRequired { reason } => Phase::RecoveryRequired(match reason {
            RecoveryReason::ExternalEffectStatusUnknown => Reason::ExternalEffectStatusUnknown,
            RecoveryReason::ComponentChildLifecycleInterrupted => {
                Reason::ComponentChildLifecycleInterrupted
            }
            RecoveryReason::ResponseCommitFailed => Reason::ResponseCommitFailed,
            RecoveryReason::CostSettlementFailed => Reason::CostSettlementFailed,
        }),
    };
    Ok(ReplayReleaseEntry {
        slot: entry.key.0,
        command_kind: record.command_kind,
        operation_id: record.operation_id,
        actor: record.actor.effective_principal,
        authentication,
        payload_hash_schema_version: record.payload_hash_schema_version,
        payload_hash: record.payload_hash,
        phase,
        created_at_ns: record.created_at_ns,
        updated_at_ns: record.updated_at_ns,
        expires_at_ns: record.expires_at_ns,
        cost_guard_settlement: record
            .cost_guard_settlement
            .map(|settlement| {
                Ok::<_, InternalError>(ReplayReleaseSettlement {
                    quota_intent_id: settlement.quota_intent_id.0,
                    reservation_intent_id: settlement.reservation_intent_id.0,
                    quota: observe_intent(settlement.quota_intent_id)?,
                    reservation: observe_intent(settlement.reservation_intent_id)?,
                })
            })
            .transpose()?,
        effect: record.effect.map(project_effect).transpose()?,
    })
}

fn observe_intent(id: IntentId) -> Result<Option<ReplayReleaseIntent>, InternalError> {
    let Some(record) = IntentStoreOps::load(id)? else {
        return Ok(None);
    };
    if record.id != id {
        return Err(InternalError::conflict());
    }
    Ok(Some(project_intent(record)))
}

pub(super) fn project_intent(record: IntentRecord) -> ReplayReleaseIntent {
    ReplayReleaseIntent {
        resource_key: record.resource_key.as_str().to_owned(),
        quantity: record.quantity,
        state: match record.state {
            IntentState::Pending => ReplayReleaseIntentState::Pending,
            IntentState::Committed => ReplayReleaseIntentState::Committed,
            IntentState::Aborted => ReplayReleaseIntentState::Aborted,
        },
        created_at_secs: record.created_at,
        ttl_secs: record.ttl_secs,
    }
}

fn project_effect(effect: ExternalEffectDescriptor) -> Result<Effect, InternalError> {
    Ok(match effect {
        ExternalEffectDescriptor::RootCanisterProvision { command_kind } => {
            bounded_identity(command_kind.as_str())?;
            Effect::RootCanisterProvision {
                command_kind: command_kind.as_str().to_owned(),
            }
        }
        ExternalEffectDescriptor::ManagementCall { canister, method } => {
            bounded_identity(&method)?;
            Effect::ManagementCall { canister, method }
        }
        ExternalEffectDescriptor::IcpTransfer { operation_id } => Effect::IcpTransfer {
            operation_id: operation_id.into_bytes(),
        },
    })
}

const fn bounded_identity(value: &str) -> Result<(), InternalError> {
    if value.len() > MAXIMUM_IDENTITY_BYTES {
        return Err(InternalError::resource_exhausted());
    }
    Ok(())
}
