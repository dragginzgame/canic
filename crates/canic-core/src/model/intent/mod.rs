//! Module: model::intent
//!
//! Responsibility: define pure receipt-backed intent identity, state, and operation results.
//! Does not own: stable storage, domain receipt validation, or external-effect execution.
//! Boundary: consumers validate domain evidence before constructing terminal evidence.

use crate::{cdk::types::Principal, ids::IntentResourceKey, model::replay::OperationId};
use serde::{Deserialize, Serialize};

pub const PAYLOAD_BINDING_SCHEMA_VERSION: u32 = 1;
pub const RECEIPT_BACKED_INTENT_SCHEMA_VERSION: u32 = 1;
pub const TERMINAL_EVIDENCE_SCHEMA_VERSION: u32 = 1;
pub const CANIC_INTENT_RESOURCE_PREFIX: &str = "canic:";
pub const MAX_RECEIPT_BACKED_INTENT_REPLAY_WINDOW_NS: u64 = 24 * 60 * 60 * 1_000_000_000;
pub const RECEIPT_TERMINAL_OBSERVATION_GRACE_NS: u64 = MAX_RECEIPT_BACKED_INTENT_REPLAY_WINDOW_NS;

/// Pure temporal admission decision supplied by the workflow policy boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptReplayWindowDecision {
    Open,
    Closed,
    TooLong { remaining_ns: u64 },
}

/// Derive the exact safe deletion deadline for one terminal application receipt.
#[must_use]
pub const fn receipt_terminal_eligible_at(
    replay_deadline_ns: u64,
    terminal_timestamp_ns: u64,
) -> Option<u64> {
    let Some(observation_deadline_ns) =
        terminal_timestamp_ns.checked_add(RECEIPT_TERMINAL_OBSERVATION_GRACE_NS)
    else {
        return None;
    };
    Some(if replay_deadline_ns > observation_deadline_ns {
        replay_deadline_ns
    } else {
        observation_deadline_ns
    })
}

#[must_use]
pub fn is_canic_owned_intent_resource_key(resource_key: &IntentResourceKey) -> bool {
    resource_key.starts_with(CANIC_INTENT_RESOURCE_PREFIX)
}

/// Opaque, versioned binding for every field that changes an external effect.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PayloadBinding {
    pub schema_version: u32,
    pub digest: [u8; 32],
}

impl PayloadBinding {
    #[must_use]
    pub const fn new(digest: [u8; 32]) -> Self {
        Self {
            schema_version: PAYLOAD_BINDING_SCHEMA_VERSION,
            digest,
        }
    }
}

/// Consumer-validated terminal decision for one receipt-backed reservation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TerminalEvidenceDecision {
    Committed,
    RolledBack,
}

/// Bounded proof reference validated by the domain owner before settlement.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TerminalEvidence {
    pub source_canister: Principal,
    pub schema_version: u32,
    pub decision: TerminalEvidenceDecision,
    pub fingerprint: [u8; 32],
}

impl TerminalEvidence {
    #[must_use]
    pub const fn new(
        source_canister: Principal,
        decision: TerminalEvidenceDecision,
        fingerprint: [u8; 32],
    ) -> Self {
        Self {
            source_canister,
            schema_version: TERMINAL_EVIDENCE_SCHEMA_VERSION,
            decision,
            fingerprint,
        }
    }
}

/// Durable lifecycle of one receipt-backed intent.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ReceiptBackedIntentState {
    Pending,
    Committed { evidence: TerminalEvidence },
    RolledBack { evidence: TerminalEvidence },
}

/// Read-only projection of one receipt-backed intent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReceiptBackedIntent {
    pub schema_version: u32,
    pub operation_id: OperationId,
    pub payload_binding: PayloadBinding,
    pub resource_key: IntentResourceKey,
    pub quantity: u64,
    pub state: ReceiptBackedIntentState,
    pub revision: u64,
    pub created_at_ns: u64,
    pub updated_at_ns: u64,
}

/// Complete input for beginning one locally decidable, expirable intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeginLocalIntentInput {
    pub resource_key: IntentResourceKey,
    pub quantity: u64,
    pub ttl_secs: Option<u64>,
    pub reservation_limit: Option<u64>,
}

/// Complete input for idempotently beginning or loading one reservation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeginReceiptBackedIntentInput {
    pub operation_id: OperationId,
    pub payload_binding: PayloadBinding,
    pub resource_key: IntentResourceKey,
    pub quantity: u64,
    pub reservation_limit: u64,
    pub replay_deadline_ns: u64,
}

/// Internal placement admission that deliberately has no application replay deadline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeginPlacementReceiptBackedIntentInput {
    pub operation_id: OperationId,
    pub payload_binding: PayloadBinding,
    pub resource_key: IntentResourceKey,
    pub quantity: u64,
    pub reservation_limit: u64,
}

/// Idempotent outcome of receipt-backed begin-or-load.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BeginReceiptBackedIntentResult {
    Created {
        revision: u64,
    },
    ExistingPending {
        revision: u64,
    },
    ExistingCommitted {
        revision: u64,
        evidence: TerminalEvidence,
    },
    ExistingRolledBack {
        revision: u64,
        evidence: TerminalEvidence,
    },
    BindingConflict,
    ReplayWindowClosed {
        replay_deadline_ns: u64,
    },
    ReplayWindowTooLong {
        remaining_ns: u64,
        maximum_ns: u64,
    },
    CapacityExceeded {
        current_quantity: u64,
        requested_quantity: u64,
        limit: u64,
    },
    StoreCapacityReached {
        current_records: u64,
        limit: u64,
    },
}

/// Complete input for compare-and-set terminal settlement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettleReceiptBackedIntentInput {
    pub operation_id: OperationId,
    pub expected_revision: u64,
    pub expected_payload_binding: PayloadBinding,
    pub evidence: TerminalEvidence,
}

/// Outcome of receipt-backed compare-and-set settlement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettleReceiptBackedIntentResult {
    Settled {
        revision: u64,
        state: ReceiptBackedIntentState,
    },
    AlreadySettled {
        revision: u64,
        state: ReceiptBackedIntentState,
    },
    NotFound,
    RevisionConflict {
        actual_revision: u64,
    },
    BindingConflict,
}

/// Complete input for deleting terminal evidence after its external receipt is released.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoveTerminalReceiptBackedIntentInput {
    pub operation_id: OperationId,
    pub expected_revision: u64,
    pub expected_payload_binding: PayloadBinding,
}

/// Outcome of deleting one exact terminal receipt-backed intent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoveTerminalReceiptBackedIntentResult {
    Removed,
    NotFound,
    NotTerminal,
    RevisionConflict { actual_revision: u64 },
    BindingConflict,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_eligibility_preserves_both_retention_deadlines() {
        assert_eq!(
            receipt_terminal_eligible_at(100, 50),
            Some(50 + RECEIPT_TERMINAL_OBSERVATION_GRACE_NS)
        );
        assert_eq!(
            receipt_terminal_eligible_at(u64::MAX - 1, 50),
            Some(u64::MAX - 1)
        );
        assert_eq!(receipt_terminal_eligible_at(u64::MAX, u64::MAX), None);
    }
}
