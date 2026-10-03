//! Module: view::intent_release
//!
//! Responsibility: retain canonical accounting rows for bounded discovery.
//! Does not own: storage, DTO conversion, settlement or release policy.
//! Boundary: scans retain primary records without requiring derived cleanup indexes.

use crate::{
    ids::IntentId,
    model::replay::OperationId,
    storage::stable::intent::{IntentRecord, ReceiptBackedIntentRecord},
};

/// Original store-qualified cursor, independent of wire representation.
#[derive(Clone, Copy)]
pub enum IntentReleaseCursor {
    Local(IntentId),
    ReceiptBacked(OperationId),
}

/// At most one bounded primary record loaded by discovery.
pub enum IntentReleaseRow {
    Local(IntentRecord),
    ReceiptBacked(ReceiptBackedIntentRecord),
}

/// A primary row and key-only evidence that another row follows.
pub struct IntentReleasePage {
    pub entry: Option<IntentReleaseRow>,
    pub has_more: bool,
}
