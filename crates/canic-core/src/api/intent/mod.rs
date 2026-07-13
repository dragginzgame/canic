//! Module: api::intent
//!
//! Responsibility: expose direct local and receipt-backed intent operations.
//! Does not own: external calls, domain receipt validation, or retry policy.
//! Boundary: maps public inputs to non-awaiting intent workflows.

use crate::{
    dto::error::Error,
    workflow::runtime::intent::{LocalIntentWorkflow, ReceiptBackedIntentWorkflow},
};

pub use crate::{
    ids::{IntentId, IntentResourceKey},
    model::{
        intent::{
            BeginLocalIntentInput, BeginReceiptBackedIntentInput, BeginReceiptBackedIntentResult,
            PayloadBinding, ReceiptBackedIntent, ReceiptBackedIntentState,
            SettleReceiptBackedIntentInput, SettleReceiptBackedIntentResult, TerminalEvidence,
            TerminalEvidenceDecision,
        },
        replay::OperationId,
    },
};

/// Direct operations for locally decidable, expirable reservations.
pub struct LocalIntentApi;

impl LocalIntentApi {
    pub fn begin(input: BeginLocalIntentInput) -> Result<IntentId, Error> {
        LocalIntentWorkflow::begin(input).map_err(Error::from)
    }

    pub fn commit(intent_id: IntentId) -> Result<(), Error> {
        LocalIntentWorkflow::commit(intent_id).map_err(Error::from)
    }

    pub fn rollback(intent_id: IntentId) -> Result<(), Error> {
        LocalIntentWorkflow::rollback(intent_id).map_err(Error::from)
    }
}

/// Exact-key receipt-backed operations for consumer-validated evidence.
pub struct ReceiptBackedIntentApi;

impl ReceiptBackedIntentApi {
    pub fn begin_or_load(
        input: &BeginReceiptBackedIntentInput,
    ) -> Result<BeginReceiptBackedIntentResult, Error> {
        ReceiptBackedIntentWorkflow::begin_or_load(input).map_err(Error::from)
    }

    pub fn load(operation_id: OperationId) -> Result<Option<ReceiptBackedIntent>, Error> {
        ReceiptBackedIntentWorkflow::load(operation_id).map_err(Error::from)
    }

    pub fn settle_if_pending(
        input: &SettleReceiptBackedIntentInput,
    ) -> Result<SettleReceiptBackedIntentResult, Error> {
        ReceiptBackedIntentWorkflow::settle_if_pending(input).map_err(Error::from)
    }
}
