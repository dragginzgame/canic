use super::*;
use crate::{
    domain::metrics::MetricsKind,
    model::public_metrics::PublicMetricsCache,
    ops::{
        runtime::metrics::{self, cycles_funding::CyclesFundingDeniedReason},
        storage::icp_refill::{
            IcpRefillMetricErrorCount, IcpRefillMetricStatusCount, IcpRefillMetricTargetTotal,
        },
    },
    storage::stable::icp_refill::{IcpRefillRecordErrorCode, IcpRefillRecordStatus},
};
use candid::Principal;

#[test]
fn public_funding_keeps_global_totals_without_child_dimensions_or_starvation() {
    metrics::reset_for_tests();
    for id in 0_u64..300 {
        let child = Principal::from_slice(&id.to_le_bytes());
        CyclesFundingMetrics::record_requested(child, 2);
        CyclesFundingMetrics::record_granted(child, 3);
        CyclesFundingMetrics::record_denied(child, 1, CyclesFundingDeniedReason::ChildNotFound);
    }
    CyclesFundingMetrics::record_denied(
        Principal::anonymous(),
        5,
        CyclesFundingDeniedReason::KillSwitchDisabled,
    );
    let rows = sample().unwrap();
    assert!(rows.iter().all(|row| row.canister_id.is_none()));
    for (name, value) in [
        ("cycles_requested_total", 600),
        ("cycles_granted_total", 900),
        ("cycles_denied_total", 305),
        ("cycles_denied_global_kill_switch.kill_switch_disabled", 5),
    ] {
        let row = rows
            .iter()
            .find(|row| row.name == format!("cycles_funding.{name}"))
            .unwrap();
        assert_eq!(row.value, value);
        assert_eq!(row.unit, "cycles");
        assert!(matches!(
            row.kind,
            PublicMetricKind::Counter {
                window_id: 0,
                saturated: false
            }
        ));
    }
    assert_eq!(
        rows.iter()
            .filter(|row| row.name.starts_with("cycles_funding."))
            .count(),
        4
    );
    PublicMetricsCache::replace(
        crate::domain::public_metrics::PublicMetricFamily::Operations,
        1,
        rows,
    )
    .unwrap();
    assert!(
        metrics::entries(MetricsKind::Core)
            .iter()
            .any(|row| row.principal.is_some())
    );
    CyclesFundingMetrics::reset();
    CyclesFundingMetrics::record_requested(Principal::anonymous(), u128::MAX);
    let capped = sample().unwrap();
    let row = capped
        .iter()
        .find(|row| row.name == "cycles_funding.cycles_requested_total")
        .unwrap();
    assert_eq!(row.value, u128::MAX);
    assert!(matches!(
        row.kind,
        PublicMetricKind::Counter {
            window_id: 0,
            saturated: true
        }
    ));
}

#[test]
fn public_refill_counts_are_unique_gauges_without_target_amounts() {
    let snapshot = IcpRefillMetricSnapshot {
        statuses: vec![
            IcpRefillMetricStatusCount {
                trigger_label: "automatic",
                phase: "notify",
                status: IcpRefillRecordStatus::Completed,
                count: 300,
            },
            IcpRefillMetricStatusCount {
                trigger_label: "manual",
                phase: "transfer",
                status: IcpRefillRecordStatus::Failed,
                count: 2,
            },
        ],
        errors: vec![IcpRefillMetricErrorCount {
            error_code: IcpRefillRecordErrorCode::BadFee,
            count: 1,
        }],
        targets: vec![IcpRefillMetricTargetTotal {
            target_canister: Principal::anonymous(),
            amount_e8s: 500,
            cycles_sent: Some(999),
        }],
    };
    let rows = refill_rows(&snapshot);
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().all(|row| row.canister_id.is_none()
        && row.unit == "count"
        && matches!(row.kind, PublicMetricKind::Gauge)));
    assert!(rows.iter().any(|row| row.name
        == "cycles_funding.icp_refill.automatic.notify.status.completed"
        && row.value == 300));
    PublicMetricsCache::replace(
        crate::domain::public_metrics::PublicMetricFamily::Operations,
        1,
        rows,
    )
    .unwrap();
}

#[test]
fn scalar_count_caps_remain_conservative_counter_quality() {
    let row = count("lifecycle.init.root.runtime.started".into(), u64::MAX);
    assert_eq!(row.value, u128::from(u64::MAX));
    assert!(matches!(
        row.kind,
        PublicMetricKind::Counter {
            window_id: 0,
            saturated: true
        }
    ));
    let zero = count("cycles_topup.request_scheduled".into(), 0);
    assert_eq!(zero.value, 0);
    assert!(matches!(
        zero.kind,
        PublicMetricKind::Counter {
            window_id: 0,
            saturated: false
        }
    ));
}
