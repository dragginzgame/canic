//! Module: view::intent
//!
//! Responsibility: define read-only intent-store scan projections.
//! Does not own: stable storage, mutation authority, or workflow decisions.
//! Boundary: storage ops return bounded pages to durable cleanup workflows.

use crate::model::{intent::ReceiptBackedIntent, replay::OperationId};

/// Read-only maintained application receipt capacity and retention projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApplicationReceiptCapacityView {
    pub total_records: u64,
    pub application_records: u64,
    pub canic_owned_records: u64,
    pub pending_records: u64,
    pub terminal_records: u64,
    pub record_limit: u64,
    pub remaining_record_headroom: u64,
    pub reserved_terminal_slots: u64,
    pub reserved_terminal_pages: u64,
    pub next_eligibility_at_ns: Option<u64>,
}

/// Result of one bounded exact application-receipt reclamation batch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApplicationReceiptReclamationBatch {
    pub removed_records: u64,
    pub next_eligibility_at_ns: Option<u64>,
}

/// Bounded placement-only acknowledgement page used by its durable cleanup owner.
#[derive(Debug)]
pub struct PlacementAcknowledgementPage {
    pub intents: Vec<ReceiptBackedIntent>,
    pub next_cursor: Option<OperationId>,
}
