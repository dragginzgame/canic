//! Module: view::intent
//!
//! Responsibility: define read-only intent-store scan projections.
//! Does not own: stable storage, mutation authority, or workflow decisions.
//! Boundary: storage ops return bounded pages to durable cleanup workflows.

use crate::model::{intent::ReceiptBackedIntent, replay::OperationId};

/// Bounded stable-map scan page used by internal durable cleanup owners.
pub struct ReceiptBackedIntentPage {
    pub intents: Vec<ReceiptBackedIntent>,
    pub next_cursor: Option<OperationId>,
}
