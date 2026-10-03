//! Module: ops::runtime::release_intents
//!
//! Responsibility: project canonical accounting obligations for controller discovery.
//! Does not own: authentication, owner reconciliation, pruning or reset authority.
//! Boundary: original identities and retained evidence survive without replay links.

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    dto::release_intents::{
        IntentReleaseEntry, IntentReleaseEvidence, IntentReleaseKey, IntentReleaseReceipt,
        IntentReleaseReceiptState, IntentReleaseResponse,
    },
    ids::IntentId,
    model::{
        intent::{ReceiptBackedIntentState, TerminalEvidence},
        replay::OperationId,
    },
    ops::{runtime::release_receipts::project_intent, storage::intent::release},
    view::intent_release::{IntentReleaseCursor, IntentReleaseRow},
};
use candid::Principal;

/// Observe one canonical row, without interpreting expiry or absent replay links as settlement.
pub fn observe(
    owner: Principal,
    start_after: Option<IntentReleaseKey>,
) -> Result<IntentReleaseResponse, InternalError> {
    let cursor = start_after.map(|key| match key {
        IntentReleaseKey::Local(id) => IntentReleaseCursor::Local(IntentId(id)),
        IntentReleaseKey::ReceiptBacked(id) => {
            IntentReleaseCursor::ReceiptBacked(OperationId::from_bytes(id))
        }
    });
    let page = release::page(cursor)?;
    let projected = page.entry.map(project);
    let next_after = projected
        .as_ref()
        .filter(|_| page.has_more)
        .map(|(key, _)| *key);
    Ok(IntentReleaseResponse {
        owner,
        entry: projected.map(|(_, entry)| entry),
        next_after,
    })
}

fn project(row: IntentReleaseRow) -> (IntentReleaseKey, IntentReleaseEntry) {
    match row {
        IntentReleaseRow::Local(record) => (
            IntentReleaseKey::Local(record.id.0),
            IntentReleaseEntry::Local {
                intent_id: record.id.0,
                record: project_intent(record),
            },
        ),
        IntentReleaseRow::ReceiptBacked(record) => (
            IntentReleaseKey::ReceiptBacked(record.operation_id.into_bytes()),
            IntentReleaseEntry::ReceiptBacked(IntentReleaseReceipt {
                operation_id: record.operation_id.into_bytes(),
                payload_hash_schema_version: record.payload_binding.schema_version,
                payload_hash: record.payload_binding.digest,
                resource_key: record.resource_key.as_str().to_owned(),
                quantity: record.quantity,
                state: match record.state {
                    ReceiptBackedIntentState::Pending => IntentReleaseReceiptState::Pending,
                    ReceiptBackedIntentState::Committed { evidence } => {
                        IntentReleaseReceiptState::Committed(project_evidence(evidence))
                    }
                    ReceiptBackedIntentState::RolledBack { evidence } => {
                        IntentReleaseReceiptState::RolledBack(project_evidence(evidence))
                    }
                },
                revision: record.revision,
                created_at_ns: record.created_at_ns,
                updated_at_ns: record.updated_at_ns,
                application_replay_deadline_ns: record
                    .application_retention
                    .map(|retention| retention.replay_deadline_ns),
            }),
        ),
    }
}

const fn project_evidence(evidence: TerminalEvidence) -> IntentReleaseEvidence {
    IntentReleaseEvidence {
        source_canister: evidence.source_canister,
        schema_version: evidence.schema_version,
        fingerprint: evidence.fingerprint,
    }
}
