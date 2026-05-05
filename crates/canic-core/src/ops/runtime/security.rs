use crate::{
    InternalError,
    cdk::types::Principal,
    dto::security::{SecurityEvent, SecurityEventReason},
    ops::runtime::RuntimeOpsError,
    storage::{
        StorageError,
        stable::security::{SecurityEventReasonRecord, SecurityEventRecord, SecurityEventStore},
    },
};
use thiserror::Error as ThisError;

pub const SECURITY_EVENT_MAX_ENTRIES: usize = 1_024;

///
/// SecurityOpsError
///

#[derive(Debug, ThisError)]
pub enum SecurityOpsError {
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl From<SecurityOpsError> for InternalError {
    fn from(err: SecurityOpsError) -> Self {
        RuntimeOpsError::SecurityOps(err).into()
    }
}

///
/// SecurityEventInput
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityEventInput {
    pub caller: Principal,
    pub endpoint: String,
    pub request_bytes: u64,
    pub max_bytes: u64,
    pub created_at: u64,
    pub reason: SecurityEventReason,
}

///
/// SecurityOps
///

pub struct SecurityOps;

impl SecurityOps {
    /// Record one stable security event for replicated execution paths.
    pub fn record(input: SecurityEventInput) -> Result<u64, InternalError> {
        let id = next_event_id();
        let entry = input_to_record(id, input);
        SecurityEventStore::append(SECURITY_EVENT_MAX_ENTRIES, entry)
            .map_err(SecurityOpsError::from)
            .map_err(InternalError::from)
    }

    /// Return newest-first security events for operator inspection.
    #[must_use]
    pub fn snapshot_newest_first() -> Vec<SecurityEvent> {
        let mut events = SecurityEventStore::snapshot()
            .into_iter()
            .map(record_to_event)
            .collect::<Vec<_>>();
        events.reverse();

        events
    }
}

// Return the next stable event id from retained security rows.
fn next_event_id() -> u64 {
    SecurityEventStore::snapshot()
        .into_iter()
        .map(|event| event.id)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

// Convert a runtime security event input into its stable record shape.
fn input_to_record(id: u64, input: SecurityEventInput) -> SecurityEventRecord {
    SecurityEventRecord {
        id,
        created_at: input.created_at,
        caller: input.caller,
        endpoint: input.endpoint,
        request_bytes: input.request_bytes,
        max_bytes: input.max_bytes,
        reason: reason_to_record(input.reason),
    }
}

// Convert a stable record into its query DTO shape.
fn record_to_event(record: SecurityEventRecord) -> SecurityEvent {
    SecurityEvent {
        id: record.id,
        created_at: record.created_at,
        caller: record.caller,
        endpoint: record.endpoint,
        request_bytes: record.request_bytes,
        max_bytes: record.max_bytes,
        reason: reason_to_dto(record.reason),
    }
}

// Convert a query DTO reason into a stable security reason.
const fn reason_to_record(reason: SecurityEventReason) -> SecurityEventReasonRecord {
    match reason {
        SecurityEventReason::IngressPayloadLimitExceeded => {
            SecurityEventReasonRecord::IngressPayloadLimitExceeded
        }
    }
}

// Convert a stable security reason into its query DTO shape.
const fn reason_to_dto(reason: SecurityEventReasonRecord) -> SecurityEventReason {
    match reason {
        SecurityEventReasonRecord::IngressPayloadLimitExceeded => {
            SecurityEventReason::IngressPayloadLimitExceeded
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdk::types::Principal;

    // Build a deterministic principal for event assertions.
    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn security_record_mapping_preserves_event_fields() {
        let record = input_to_record(
            7,
            SecurityEventInput {
                caller: p(1),
                endpoint: "save".to_string(),
                request_bytes: 32,
                max_bytes: 16,
                created_at: 12,
                reason: SecurityEventReason::IngressPayloadLimitExceeded,
            },
        );
        let event = record_to_event(record);

        assert_eq!(event.id, 7);
        assert_eq!(event.caller, p(1));
        assert_eq!(event.endpoint, "save");
        assert_eq!(event.request_bytes, 32);
        assert_eq!(event.max_bytes, 16);
        assert_eq!(event.created_at, 12);
        assert_eq!(
            event.reason,
            SecurityEventReason::IngressPayloadLimitExceeded
        );
    }
}
