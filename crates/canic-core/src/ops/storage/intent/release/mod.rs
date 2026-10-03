//! Module: ops::storage::intent::release
//!
//! Responsibility: read one canonical accounting row with bounded lookahead.
//! Does not own: cleanup-index repair, settlement or release admission.
//! Boundary: retain terminal/expired records and validate exact primary identities.

use super::{
    ensure_receipt_backed_record_schema, ensure_schema, validate_payload_binding,
    validate_terminal_evidence,
};
use crate::{
    InternalError,
    model::intent::{ReceiptBackedIntentState, TerminalEvidenceDecision},
    storage::stable::intent::{IntentStore, ReceiptBackedIntentStore},
    view::intent_release::{IntentReleaseCursor, IntentReleasePage, IntentReleaseRow},
};
use std::ops::Bound::{Excluded, Unbounded};

/// Read at most one value from either canonical store; never decode lookahead values.
pub fn page(cursor: Option<IntentReleaseCursor>) -> Result<IntentReleasePage, InternalError> {
    ensure_schema()?;
    let receipt_cursor = if let Some(IntentReleaseCursor::ReceiptBacked(id)) = cursor {
        Some(id)
    } else {
        let local_cursor = match cursor {
            Some(IntentReleaseCursor::Local(id)) => Some(id),
            _ => None,
        };
        let local = IntentStore::with_records(|records| {
            let mut rows = records.range((local_cursor.map_or(Unbounded, Excluded), Unbounded));
            let entry = rows.next().map(|row| (*row.key(), row.value()));
            (entry, rows.next().is_some())
        });
        if let (Some((key, record)), has_more) = local {
            if key != record.id {
                return Err(InternalError::conflict());
            }
            return Ok(IntentReleasePage {
                entry: Some(IntentReleaseRow::Local(record)),
                has_more: has_more || ReceiptBackedIntentStore::len() > 0,
            });
        }
        None
    };
    ReceiptBackedIntentStore::with_records(|records| {
        let mut rows = records.range((receipt_cursor.map_or(Unbounded, Excluded), Unbounded));
        let entry = rows
            .next()
            .map(|row| {
                let record = row.value();
                if *row.key() != record.operation_id {
                    return Err(InternalError::conflict());
                }
                ensure_receipt_backed_record_schema(&record)?;
                validate_payload_binding(record.payload_binding)?;
                let terminal = match record.state {
                    ReceiptBackedIntentState::Pending => None,
                    ReceiptBackedIntentState::Committed { evidence } => {
                        Some((evidence, TerminalEvidenceDecision::Committed))
                    }
                    ReceiptBackedIntentState::RolledBack { evidence } => {
                        Some((evidence, TerminalEvidenceDecision::RolledBack))
                    }
                };
                if let Some((evidence, expected)) = terminal {
                    validate_terminal_evidence(evidence.schema_version)?;
                    if evidence.decision != expected {
                        return Err(InternalError::conflict());
                    }
                }
                Ok::<_, InternalError>(IntentReleaseRow::ReceiptBacked(record))
            })
            .transpose()?;
        Ok(IntentReleasePage {
            entry,
            has_more: rows.next().is_some(),
        })
    })
}
