//! Outer relay frames preserve exact targets and reject inner-hop or other-operation frames.

use super::*;
use crate::dto::{page::PageRequest, role::MetricsStatusRequest, root_observation::Request};
use ic_platform::Principal;
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
