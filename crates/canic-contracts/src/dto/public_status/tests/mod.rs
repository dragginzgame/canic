//! Qualify native History payloads before switching the deployed transport.

use super::*;

#[test]
fn native_history_preserves_full_retention_and_counter_lifetimes() {
    // The maintained source cache retains 288 five-minute observations.
    let point = PublicHistoryPoint {
        delta: Some(PublicCounterDelta {
            amount: u128::MAX,
            elapsed_ns: u64::MAX,
        }),
        slot_start_ns: u64::MAX,
        observed_at_ns: u64::MAX,
        value: u128::MAX,
        kind: PublicMetricKind::TimerCounter {
            registration: TimerMetricRegistration {
                canister_version: u64::MAX,
                started_at_ns: u64::MAX,
                sequence: u64::MAX,
            },
            saturated: true,
        },
    };
    let snapshot = PublicHistorySnapshot {
        state: PublicSnapshotState::Stale,
        unit: Some("u".repeat(128)),
        heap_started_at_ns: Some(u64::MAX),
        canister_version: u64::MAX,
        coverage_start_ns: Some(u64::MAX),
        cadence_ns: u64::MAX,
        retention_ns: u64::MAX,
        stale_after_ns: u64::MAX,
        truncated: true,
        series_limit: u64::MAX,
        byte_limit: u64::MAX,
        reserved_bytes: u64::MAX,
        points: Page {
            entries: vec![point; 288],
            total: 288,
        },
    };
    let limits = crate::protocol::public_history::RESPONSE_LIMITS;
    let method_id = crate::protocol::public_history::METHOD_ID;
    let frame = ic_wire::native::encode_response(method_id, &snapshot, limits).unwrap();
    let decoded: PublicHistorySnapshot =
        ic_wire::native::decode_response(method_id, &frame, limits).unwrap();
    assert_eq!(decoded.points.entries, snapshot.points.entries);
    assert_eq!(decoded.points.total, snapshot.points.total);
    // Compare all metadata too, including deadlines, coverage and capacity.
    assert_eq!(
        ic_wire::native::encode_response(method_id, &decoded, limits).unwrap(),
        frame
    );
    assert!(matches!(
        ic_wire::native::decode_response::<PublicHistorySnapshot>(method_id + 1, &frame, limits),
        Err(ic_wire::Error::Identity)
    ));
    assert!(matches!(
        ic_wire::native::decode_response::<PublicHistorySnapshot>(
            method_id,
            &frame,
            ic_wire::Limits {
                bytes: frame.len() - 1,
                ..limits
            }
        ),
        Err(ic_wire::Error::Limit)
    ));
    assert!(ic_wire::typescript::native_schema::<PublicHistorySnapshot>().is_ok());
}

#[test]
fn history_selector_uses_native_principal_and_bounded_frames() {
    use crate::protocol::public_history::{METHOD_ID, REQUEST_LIMITS};
    let request = PublicHistoryRequest {
        family: PublicMetricFamily::Performance,
        name: "x".repeat(128),
        canister_id: Some(ic_platform::Principal::from_slice(&[42; 29])),
        page: PageRequest {
            offset: u64::MAX,
            limit: u64::MAX,
        },
    };
    let frame = ic_wire::native::encode_request(METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    let decoded: PublicHistoryRequest =
        ic_wire::native::decode_request(METHOD_ID, &frame, REQUEST_LIMITS).unwrap();
    assert_eq!(decoded.family, request.family);
    assert_eq!(decoded.name, request.name);
    assert_eq!(decoded.canister_id, request.canister_id);
    assert_eq!(decoded.page, request.page);
    let schema = ic_wire::typescript::native_schema::<PublicHistoryRequest>().unwrap();
    assert_eq!(
        schema["fields"][2]["value"],
        ic_wire::typescript::native_schema::<Option<ic_platform::Principal>>().unwrap()
    );
}
