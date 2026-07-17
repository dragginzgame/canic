//! Module: ops::runtime::metrics::icp_refill
//!
//! Responsibility: record and snapshot low-cardinality runtime metrics for the icp_refill family.
//! Does not own: workflow decisions, persisted records, or endpoint DTOs.
//! Boundary: ops-layer metrics consumed by workflow metrics projection.

use crate::{
    cdk::types::Principal,
    dto::metrics::{MetricEntry, MetricValue},
    ops::storage::icp_refill::{IcpRefillMetricSnapshot, IcpRefillRecordOps},
    storage::stable::icp_refill::{IcpRefillRecordErrorCode, IcpRefillRecordStatus},
};

///
/// IcpRefillMetrics
///
/// Operations-layer projector for persisted ICP refill records.
///

pub struct IcpRefillMetrics;

impl IcpRefillMetrics {
    #[must_use]
    pub fn entries() -> Vec<MetricEntry> {
        entries_from_snapshot(&IcpRefillRecordOps::metric_snapshot())
    }
}

pub(super) fn entries_from_snapshot(snapshot: &IcpRefillMetricSnapshot) -> Vec<MetricEntry> {
    let mut entries = snapshot
        .statuses
        .iter()
        .map(|status| {
            count_entry(
                vec![
                    "icp_refill".to_string(),
                    record_phase(status.status, status.error_code).to_string(),
                    "status".to_string(),
                    status_label(status.status).to_string(),
                ],
                status.count,
            )
        })
        .collect::<Vec<_>>();
    entries.extend(snapshot.errors.iter().map(|error| {
        count_entry(
            vec![
                "icp_refill".to_string(),
                error_phase(error.error_code).to_string(),
                "error".to_string(),
                error_label(error.error_code).to_string(),
            ],
            error.count,
        )
    }));
    entries.extend(snapshot.targets.iter().map(|target| {
        principal_u128_entry(
            vec![
                "icp_refill".to_string(),
                "transfer".to_string(),
                "amount_e8s".to_string(),
                "target".to_string(),
            ],
            target.target_canister,
            target.amount_e8s,
        )
    }));
    entries.extend(snapshot.targets.iter().filter_map(|target| {
        target.cycles_sent.map(|cycles_sent| {
            principal_u128_entry(
                vec![
                    "icp_refill".to_string(),
                    "notify".to_string(),
                    "cycles_sent".to_string(),
                    "target".to_string(),
                ],
                target.target_canister,
                cycles_sent,
            )
        })
    }));
    entries
}

const fn count_entry(labels: Vec<String>, count: u64) -> MetricEntry {
    MetricEntry {
        labels,
        principal: None,
        value: MetricValue::Count(count),
    }
}

const fn principal_u128_entry(
    labels: Vec<String>,
    principal: Principal,
    value: u128,
) -> MetricEntry {
    MetricEntry {
        labels,
        principal: Some(principal),
        value: MetricValue::U128(value),
    }
}

const fn record_phase(
    status: IcpRefillRecordStatus,
    error_code: Option<IcpRefillRecordErrorCode>,
) -> &'static str {
    match status {
        IcpRefillRecordStatus::Requested => "preflight",
        IcpRefillRecordStatus::Transferred
        | IcpRefillRecordStatus::NotifyProcessing
        | IcpRefillRecordStatus::Completed
        | IcpRefillRecordStatus::InvalidTransaction
        | IcpRefillRecordStatus::Refunded
        | IcpRefillRecordStatus::TransactionTooOld => "notify",
        IcpRefillRecordStatus::Failed => match error_code {
            Some(error_code) => error_phase(error_code),
            None => "transfer",
        },
    }
}

const fn error_phase(error: IcpRefillRecordErrorCode) -> &'static str {
    match error {
        IcpRefillRecordErrorCode::RateGateDenied | IcpRefillRecordErrorCode::RequestDenied => {
            "preflight"
        }
        IcpRefillRecordErrorCode::BadFee
        | IcpRefillRecordErrorCode::Duplicate
        | IcpRefillRecordErrorCode::InvalidLedgerBlockIndex
        | IcpRefillRecordErrorCode::LedgerTransferFailed
        | IcpRefillRecordErrorCode::TransferWindowStale => "transfer",
        IcpRefillRecordErrorCode::InvalidTransaction
        | IcpRefillRecordErrorCode::NotifyFailed
        | IcpRefillRecordErrorCode::NotifyMaxAttempts
        | IcpRefillRecordErrorCode::Processing
        | IcpRefillRecordErrorCode::Refunded
        | IcpRefillRecordErrorCode::TransactionTooOld => "notify",
        IcpRefillRecordErrorCode::FabricationUnavailable => "fabricate",
    }
}

const fn status_label(status: IcpRefillRecordStatus) -> &'static str {
    match status {
        IcpRefillRecordStatus::Completed => "completed",
        IcpRefillRecordStatus::Failed => "failed",
        IcpRefillRecordStatus::InvalidTransaction => "invalid_transaction",
        IcpRefillRecordStatus::NotifyProcessing => "notify_processing",
        IcpRefillRecordStatus::Refunded => "refunded",
        IcpRefillRecordStatus::Requested => "requested",
        IcpRefillRecordStatus::TransactionTooOld => "transaction_too_old",
        IcpRefillRecordStatus::Transferred => "transferred",
    }
}

const fn error_label(error: IcpRefillRecordErrorCode) -> &'static str {
    match error {
        IcpRefillRecordErrorCode::BadFee => "bad_fee",
        IcpRefillRecordErrorCode::Duplicate => "duplicate",
        IcpRefillRecordErrorCode::FabricationUnavailable => "fabrication_unavailable",
        IcpRefillRecordErrorCode::InvalidLedgerBlockIndex => "invalid_ledger_block_index",
        IcpRefillRecordErrorCode::InvalidTransaction => "invalid_transaction",
        IcpRefillRecordErrorCode::LedgerTransferFailed => "ledger_transfer_failed",
        IcpRefillRecordErrorCode::NotifyFailed => "notify_failed",
        IcpRefillRecordErrorCode::NotifyMaxAttempts => "notify_max_attempts",
        IcpRefillRecordErrorCode::Processing => "processing",
        IcpRefillRecordErrorCode::RateGateDenied => "rate_gate_denied",
        IcpRefillRecordErrorCode::Refunded => "refunded",
        IcpRefillRecordErrorCode::RequestDenied => "request_denied",
        IcpRefillRecordErrorCode::TransactionTooOld => "transaction_too_old",
        IcpRefillRecordErrorCode::TransferWindowStale => "transfer_window_stale",
    }
}
