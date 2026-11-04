//! Outer relay frames preserve exact targets and reject inner-hop or other-operation frames.

use super::*;
use crate::dto::{page::PageRequest, role::MetricsStatusRequest, root_observation::Request};
use ic_platform_types::Principal;
use ic_wire::{Error, native};
use serde::{Serialize, de::DeserializeOwned};

fn round_trip<T: Serialize + DeserializeOwned>(id: u64, input: T) {
    let request = Request {
        canister_id: Principal::from_slice(&[9; 29]),
        input,
    };
    let bytes = native::encode_request(id, &request, REQUEST_LIMITS).unwrap();
    let decoded: Request<T> = native::decode_request(id, &bytes, REQUEST_LIMITS).unwrap();
    assert_eq!(decoded.canister_id, request.canister_id);
    assert_eq!(
        native::encode_request(id, &decoded, REQUEST_LIMITS).unwrap(),
        bytes
    );
    assert!(matches!(
        native::decode_response::<Request<T>>(id, &bytes, REQUEST_LIMITS),
        Err(Error::Identity)
    ));
    let mut trailing = bytes;
    trailing.push(0);
    assert!(matches!(
        native::decode_request::<Request<T>>(id, &trailing, REQUEST_LIMITS),
        Err(Error::Trailing)
    ));
}

#[test]
fn maximal_targets_and_operation_inputs_fit_the_outer_budget() {
    round_trip(child_funding::METHOD_ID, Principal::from_slice(&[7; 29]));
    round_trip(cycle_balance::METHOD_ID, ());
    let page = PageRequest {
        limit: u64::MAX,
        offset: u64::MAX,
    };
    round_trip(cycle_history::METHOD_ID, page);
    round_trip(cycle_topups::METHOD_ID, page);
    round_trip(memory_allocations::METHOD_ID, ());
    round_trip(
        metrics::METHOD_ID,
        MetricsStatusRequest {
            kind: crate::values::metrics::MetricsKind::Security,
            page,
        },
    );
}

#[test]
fn operation_and_hop_identities_are_not_interchangeable() {
    use crate::protocol::observability_relay as inner;
    let operations = [
        (
            child_funding::METHOD,
            child_funding::METHOD_ID,
            inner::child_funding::METHOD_ID,
        ),
        (
            cycle_balance::METHOD,
            cycle_balance::METHOD_ID,
            inner::cycle_balance::METHOD_ID,
        ),
        (
            cycle_history::METHOD,
            cycle_history::METHOD_ID,
            inner::cycle_history::METHOD_ID,
        ),
        (
            cycle_topups::METHOD,
            cycle_topups::METHOD_ID,
            inner::cycle_topups::METHOD_ID,
        ),
        (
            memory_allocations::METHOD,
            memory_allocations::METHOD_ID,
            inner::memory_allocations::METHOD_ID,
        ),
        (
            metrics::METHOD,
            metrics::METHOD_ID,
            inner::metrics::METHOD_ID,
        ),
    ];
    let mut names = std::collections::BTreeSet::new();
    let mut ids = std::collections::BTreeSet::new();
    for (name, id, inner_id) in operations {
        assert!(names.insert(name));
        assert!(ids.insert(id));
        let request = Request {
            canister_id: Principal::from_slice(&[1]),
            input: (),
        };
        let bytes = native::encode_request(id, &request, REQUEST_LIMITS).unwrap();
        for other in operations
            .iter()
            .map(|(_, other, _)| *other)
            .chain([inner_id])
        {
            if other != id {
                assert!(matches!(
                    native::decode_request::<Request<()>>(other, &bytes, REQUEST_LIMITS),
                    Err(Error::Identity)
                ));
            }
        }
    }
}

#[test]
fn request_frame_budget_rejects_before_unbounded_input_is_decoded() {
    assert!(matches!(
        native::decode_request::<Request<()>>(
            cycle_balance::METHOD_ID,
            &vec![0; REQUEST_LIMITS.bytes + 1],
            REQUEST_LIMITS,
        ),
        Err(Error::Limit)
    ));
    let request = Request {
        canister_id: Principal::from_slice(&[1]),
        input: vec![0_u8; REQUEST_LIMITS.bytes],
    };
    assert!(matches!(
        native::encode_request(cycle_balance::METHOD_ID, &request, REQUEST_LIMITS),
        Err(Error::Limit)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn all_outer_payloads_have_native_schemas_without_candid_projection() {
    use crate::dto::{
        child_funding::ChildFundingUsage, cycle_topups::CycleTopupEvent, cycles::CycleTrackerEntry,
        error::Error, memory::MemoryAllocationsResponse, metric_page::MetricEntry, page::Page,
        role::CycleBalanceStatusResponse,
    };
    use ic_wire::typescript::native_schema;
    native_schema::<Request<Principal>>().unwrap();
    native_schema::<Request<()>>().unwrap();
    native_schema::<Request<PageRequest>>().unwrap();
    native_schema::<Request<MetricsStatusRequest>>().unwrap();
    native_schema::<Result<ChildFundingUsage, Error>>().unwrap();
    native_schema::<Result<CycleBalanceStatusResponse, Error>>().unwrap();
    native_schema::<Result<Page<CycleTrackerEntry>, Error>>().unwrap();
    native_schema::<Result<Page<CycleTopupEvent>, Error>>().unwrap();
    native_schema::<Result<MemoryAllocationsResponse, Error>>().unwrap();
    native_schema::<Result<Page<MetricEntry>, Error>>().unwrap();
}

#[test]
fn child_funding_reply_preserves_exact_authority_and_uncertainty_at_each_hop() {
    use crate::dto::{child_funding::ChildFundingUsage, error::Error as CanicError};

    let parent = Principal::from_slice(&[7; 29]);
    let child = Principal::from_slice(&[8; 29]);
    for reserved_cycles in [None, Some(crate::cycles::Cycles::new(u128::MAX))] {
        let usage = ChildFundingUsage {
            parent,
            child,
            observed_at_ns: u64::MAX,
            accounted_cycles: crate::cycles::Cycles::new(u128::MAX),
            last_accounted_at_secs: u64::MAX,
            pending_operations: u32::MAX,
            reserved_cycles,
        };
        for (id, limits) in [
            (
                crate::protocol::child_funding::METHOD_ID,
                crate::protocol::child_funding::RESPONSE_LIMITS,
            ),
            (
                crate::protocol::observability_relay::child_funding::METHOD_ID,
                crate::protocol::observability_relay::child_funding::RESPONSE_LIMITS,
            ),
            (child_funding::METHOD_ID, child_funding::RESPONSE_LIMITS),
        ] {
            let bytes = native::encode_response(id, &Ok::<_, CanicError>(&usage), limits).unwrap();
            let restored: Result<ChildFundingUsage, CanicError> =
                native::decode_response(id, &bytes, limits).unwrap();
            assert_eq!(restored.unwrap(), usage);
        }
    }
}

#[test]
fn topup_reply_preserves_full_range_events_and_failure_custody_at_each_hop() {
    use crate::dto::{
        cycle_topups::{CycleTopupEvent, CycleTopupFailure},
        error::Error as CanicError,
        page::Page,
    };
    use crate::{
        cycles::Cycles,
        values::cycles::{CycleTopupEventStatus, CycleTopupFailureDisposition},
    };

    let page = Page {
        entries: vec![
            CycleTopupEvent {
                timestamp_secs: u64::MAX,
                sequence: u32::MAX,
                requested_cycles: Cycles::new(u128::MAX),
                transferred_cycles: Some(Cycles::new(u128::MAX)),
                status: CycleTopupEventStatus::RequestErr,
                error: Some("parent refused 🌍".into()),
                parent_failure: Some(CycleTopupFailure {
                    parent: Principal::from_slice(&[7; 29]),
                    operation_id: [u8::MAX; 32],
                    public_error_code: u16::MAX,
                    disposition: CycleTopupFailureDisposition::Retry,
                }),
            },
            CycleTopupEvent {
                timestamp_secs: 0,
                sequence: 0,
                requested_cycles: Cycles::new(0),
                transferred_cycles: None,
                status: CycleTopupEventStatus::RequestScheduled,
                error: None,
                parent_failure: None,
            },
        ],
        total: u64::MAX,
    };
    for (id, limits) in [
        (
            crate::protocol::cycle_topups::METHOD_ID,
            crate::protocol::cycle_topups::RESPONSE_LIMITS,
        ),
        (
            crate::protocol::observability_relay::cycle_topups::METHOD_ID,
            crate::protocol::observability_relay::cycle_topups::RESPONSE_LIMITS,
        ),
        (cycle_topups::METHOD_ID, cycle_topups::RESPONSE_LIMITS),
    ] {
        let bytes = native::encode_response(id, &Ok::<_, CanicError>(&page), limits).unwrap();
        let restored: Result<Page<CycleTopupEvent>, CanicError> =
            native::decode_response(id, &bytes, limits).unwrap();
        let restored = restored.unwrap();
        assert_eq!(restored.total, page.total);
        assert_eq!(restored.entries, page.entries);
    }
}
