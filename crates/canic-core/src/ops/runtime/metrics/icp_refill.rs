//! Module: ops::runtime::metrics::icp_refill
//!
//! Responsibility: record and snapshot low-cardinality runtime metrics for the icp_refill family.
//! Does not own: workflow decisions, persisted records, or endpoint DTOs.
//! Boundary: ops-layer metrics consumed by workflow metrics projection.

use crate::{
    cdk::types::Principal,
    dto::metrics::{MetricEntry, MetricValue},
    ops::storage::icp_refill::IcpRefillRecordOps,
    storage::stable::icp_refill::{
        IcpRefillRecord, IcpRefillRecordErrorCode, IcpRefillRecordStatus,
    },
};
use std::collections::BTreeMap;

///
/// IcpRefillMetrics
///
/// Operations-layer projector for persisted ICP refill records.
///

pub struct IcpRefillMetrics;

impl IcpRefillMetrics {
    #[must_use]
    pub fn entries() -> Vec<MetricEntry> {
        entries_from_records(&IcpRefillRecordOps::records())
    }
}

pub(super) fn entries_from_records(records: &[IcpRefillRecord]) -> Vec<MetricEntry> {
    let mut status_counts = BTreeMap::<(&'static str, &'static str), u64>::new();
    let mut error_counts = BTreeMap::<(&'static str, &'static str), u64>::new();
    let mut amount_by_target = BTreeMap::<Principal, u128>::new();
    let mut cycles_by_target = BTreeMap::<Principal, u128>::new();

    for record in records {
        *status_counts
            .entry((record_phase(record), status_label(record.status)))
            .or_default() += 1;
        if let Some(error_code) = record.error_code {
            *error_counts
                .entry((error_phase(error_code), error_label(error_code)))
                .or_default() += 1;
        }
        saturating_add_principal_value(
            &mut amount_by_target,
            record.target_canister,
            u128::from(record.amount_e8s),
        );
        if record.status == IcpRefillRecordStatus::Completed
            && let Some(cycles_sent) = &record.cycles_sent
        {
            saturating_add_principal_value(
                &mut cycles_by_target,
                record.target_canister,
                IcpRefillRecordOps::nat_to_u128_saturating(cycles_sent),
            );
        }
    }

    let mut entries = status_counts
        .into_iter()
        .map(|((phase, status), count)| {
            count_entry(
                vec![
                    "icp_refill".to_string(),
                    phase.to_string(),
                    "status".to_string(),
                    status.to_string(),
                ],
                count,
            )
        })
        .collect::<Vec<_>>();
    entries.extend(error_counts.into_iter().map(|((phase, error), count)| {
        count_entry(
            vec![
                "icp_refill".to_string(),
                phase.to_string(),
                "error".to_string(),
                error.to_string(),
            ],
            count,
        )
    }));
    entries.extend(amount_by_target.into_iter().map(|(target, amount)| {
        principal_u128_entry(
            vec![
                "icp_refill".to_string(),
                "transfer".to_string(),
                "amount_e8s".to_string(),
                "target".to_string(),
            ],
            target,
            amount,
        )
    }));
    entries.extend(cycles_by_target.into_iter().map(|(target, cycles)| {
        principal_u128_entry(
            vec![
                "icp_refill".to_string(),
                "notify".to_string(),
                "cycles_sent".to_string(),
                "target".to_string(),
            ],
            target,
            cycles,
        )
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

fn saturating_add_principal_value(
    totals: &mut BTreeMap<Principal, u128>,
    principal: Principal,
    value: u128,
) {
    let entry = totals.entry(principal).or_default();
    *entry = entry.saturating_add(value);
}

const fn record_phase(record: &IcpRefillRecord) -> &'static str {
    match record.status {
        IcpRefillRecordStatus::Requested => "preflight",
        IcpRefillRecordStatus::Transferred
        | IcpRefillRecordStatus::NotifyProcessing
        | IcpRefillRecordStatus::Completed
        | IcpRefillRecordStatus::InvalidTransaction
        | IcpRefillRecordStatus::Refunded
        | IcpRefillRecordStatus::TransactionTooOld => "notify",
        IcpRefillRecordStatus::Failed => match record.error_code {
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
