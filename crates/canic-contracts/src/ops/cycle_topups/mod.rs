//! Lossless conversion from funding history projections to native transport.
use crate::dto::{cycle_topups as wire, cycles as source, page::Page};

impl From<source::CycleTopupFailure> for wire::CycleTopupFailure {
    fn from(value: source::CycleTopupFailure) -> Self {
        Self {
            parent: value.parent.into(),
            operation_id: value.operation_id,
            public_error_code: value.public_error_code,
            disposition: value.disposition,
        }
    }
}
impl From<source::CycleTopupEvent> for wire::CycleTopupEvent {
    fn from(value: source::CycleTopupEvent) -> Self {
        Self {
            timestamp_secs: value.timestamp_secs,
            sequence: value.sequence,
            requested_cycles: value.requested_cycles,
            transferred_cycles: value.transferred_cycles,
            status: value.status,
            error: value.error,
            parent_failure: value.parent_failure.map(Into::into),
        }
    }
}
impl From<Page<source::CycleTopupEvent>> for Page<wire::CycleTopupEvent> {
    fn from(value: Page<source::CycleTopupEvent>) -> Self {
        Self {
            entries: value.entries.into_iter().map(Into::into).collect(),
            total: value.total,
        }
    }
}

impl From<wire::CycleTopupFailure> for source::CycleTopupFailure {
    fn from(value: wire::CycleTopupFailure) -> Self {
        Self {
            parent: value.parent.into(),
            operation_id: value.operation_id,
            public_error_code: value.public_error_code,
            disposition: value.disposition,
        }
    }
}
impl From<wire::CycleTopupEvent> for source::CycleTopupEvent {
    fn from(value: wire::CycleTopupEvent) -> Self {
        Self {
            timestamp_secs: value.timestamp_secs,
            sequence: value.sequence,
            requested_cycles: value.requested_cycles,
            transferred_cycles: value.transferred_cycles,
            status: value.status,
            error: value.error,
            parent_failure: value.parent_failure.map(Into::into),
        }
    }
}
impl From<Page<wire::CycleTopupEvent>> for Page<source::CycleTopupEvent> {
    fn from(value: Page<wire::CycleTopupEvent>) -> Self {
        Self {
            entries: value.entries.into_iter().map(Into::into).collect(),
            total: value.total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relay_round_trip_preserves_parent_failure_and_page_evidence() {
        let event = source::CycleTopupEvent {
            timestamp_secs: u64::MAX,
            sequence: u32::MAX,
            requested_cycles: crate::cycles::Cycles::new(u128::MAX),
            transferred_cycles: Some(crate::cycles::Cycles::new(9)),
            status: crate::values::cycles::CycleTopupEventStatus::RequestErr,
            error: Some("parent refused".into()),
            parent_failure: Some(source::CycleTopupFailure {
                parent: ic_platform::principal::ExternalPrincipal::from_slice(&[7; 29]),
                operation_id: [8; 32],
                public_error_code: 31,
                disposition: crate::values::cycles::CycleTopupFailureDisposition::Retry,
            }),
        };
        let expected = crate::protocol::authority::encode(&event).unwrap();
        let wire: Page<wire::CycleTopupEvent> = Page {
            entries: vec![event],
            total: 17,
        }
        .into();
        let protocol = crate::protocol::observability_relay::cycle_topups::METHOD_ID;
        let bytes = ic_wire::native::encode_response(
            protocol,
            &wire,
            crate::protocol::observability_relay::cycle_topups::RESPONSE_LIMITS,
        )
        .unwrap();
        let decoded: Page<wire::CycleTopupEvent> = ic_wire::native::decode_response(
            protocol,
            &bytes,
            crate::protocol::observability_relay::cycle_topups::RESPONSE_LIMITS,
        )
        .unwrap();
        let restored: Page<source::CycleTopupEvent> = decoded.into();
        assert_eq!(restored.total, 17);
        assert_eq!(restored.entries.len(), 1);
        assert_eq!(
            crate::protocol::authority::encode(&restored.entries[0]).unwrap(),
            expected
        );
    }
}
