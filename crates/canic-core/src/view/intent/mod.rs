//! Module: view::intent
//!
//! Responsibility: define read-only intent-store scan projections.
//! Does not own: stable storage, mutation authority, or workflow decisions.
//! Boundary: storage ops return bounded pages to durable cleanup workflows.

use crate::model::{intent::ReceiptBackedIntent, replay::OperationId};

/// Bounded placement-only acknowledgement page used by its durable cleanup owner.
#[derive(Debug)]
pub struct PlacementAcknowledgementPage {
    pub intents: Vec<ReceiptBackedIntent>,
    pub next_cursor: Option<OperationId>,
}
