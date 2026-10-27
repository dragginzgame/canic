//! Qualify the maximum published cache page and exact framed method boundary.

use super::*;
use crate::dto::{
    page::{Page, PageRequest},
    public_status::{
        PublicMetric, PublicMetricFamily, PublicMetricKind, PublicMetricsRequest,
        PublicMetricsSnapshot, PublicSnapshotState, TimerMetricRegistration,
    },
};

#[test]
fn metrics_frames_admit_full_cache_pages_and_reject_wrong_identity_and_limits() {
    let request = PublicMetricsRequest {
        family: PublicMetricFamily::Performance,
        page: PageRequest {
            limit: 256,
            offset: u64::MAX,
        },
    };
    let frame = ic_wire::native::encode_request(METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    let decoded: PublicMetricsRequest =
        ic_wire::native::decode_request(METHOD_ID, &frame, REQUEST_LIMITS).unwrap();
    assert_eq!(decoded.family, request.family);
    assert_eq!(decoded.page, request.page);
    let row = PublicMetric {
        name: "x".repeat(128),
        canister_id: Some(ic_platform::Principal::from_slice(&[42; 29])),
        value: u128::MAX,
        unit: "y".repeat(128),
        observed_at_ns: u64::MAX,
        kind: PublicMetricKind::TimerCounter {
            registration: TimerMetricRegistration {
                canister_version: u64::MAX,
                started_at_ns: u64::MAX,
                sequence: u64::MAX,
            },
            saturated: true,
        },
    };
    let snapshot = PublicMetricsSnapshot {
        family: request.family,
        state: PublicSnapshotState::Stale,
        sampled_at_ns: Some(u64::MAX),
        stale_after_ns: u64::MAX,
        truncated: true,
        metrics: Page {
            entries: vec![row; 256],
            total: 256,
        },
    };
    let frame = ic_wire::native::encode_response(METHOD_ID, &snapshot, RESPONSE_LIMITS).unwrap();
    let decoded: PublicMetricsSnapshot =
        ic_wire::native::decode_response(METHOD_ID, &frame, RESPONSE_LIMITS).unwrap();
    assert_eq!(decoded.metrics.entries, snapshot.metrics.entries);
    assert_eq!(decoded.state, snapshot.state);
    assert!(matches!(
        ic_wire::native::decode_response::<PublicMetricsSnapshot>(
            METHOD_ID + 1,
            &frame,
            RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
    let limits = ic_wire::Limits {
        bytes: frame.len() - 1,
        ..RESPONSE_LIMITS
    };
    assert!(matches!(
        ic_wire::native::decode_response::<PublicMetricsSnapshot>(METHOD_ID, &frame, limits),
        Err(ic_wire::Error::Limit)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn metrics_schema_uses_native_principal_dimensions() {
    let row = ic_wire::typescript::native_schema::<PublicMetric>().unwrap();
    let principal = ic_wire::typescript::native_schema::<Option<ic_platform::Principal>>().unwrap();
    assert_eq!(row["fields"][1]["value"], principal);
    assert!(ic_wire::typescript::native_schema::<PublicMetricsRequest>().is_ok());
    assert!(ic_wire::typescript::native_schema::<PublicMetricsSnapshot>().is_ok());
}
