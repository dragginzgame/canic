use crate::{
    cdk::types::Principal,
    dto::{
        icp_refill::{IcpRefillErrorCode, IcpRefillStatus},
        metrics::{MetricEntry, MetricValue},
    },
    ops::storage::icp_refill::IcpRefillRecordOps,
    storage::stable::icp_refill::IcpRefillRecord,
};
use std::collections::BTreeMap;

///
/// IcpRefillMetrics
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
        if record.status == IcpRefillStatus::Completed
            && let Some(cycles_sent) = &record.cycles_sent
        {
            saturating_add_principal_value(
                &mut cycles_by_target,
                record.target_canister,
                nat_to_u128_saturating(cycles_sent),
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

fn nat_to_u128_saturating(value: &crate::cdk::candid::Nat) -> u128 {
    u128::try_from(value.0.clone()).unwrap_or(u128::MAX)
}

const fn record_phase(record: &IcpRefillRecord) -> &'static str {
    match record.status {
        IcpRefillStatus::Requested => "preflight",
        IcpRefillStatus::Transferred
        | IcpRefillStatus::NotifyProcessing
        | IcpRefillStatus::Completed
        | IcpRefillStatus::InvalidTransaction
        | IcpRefillStatus::Refunded
        | IcpRefillStatus::TransactionTooOld => "notify",
        IcpRefillStatus::Failed => match record.error_code {
            Some(error_code) => error_phase(error_code),
            None => "transfer",
        },
    }
}

const fn error_phase(error: IcpRefillErrorCode) -> &'static str {
    match error {
        IcpRefillErrorCode::RateGateDenied | IcpRefillErrorCode::RequestDenied => "preflight",
        IcpRefillErrorCode::BadFee
        | IcpRefillErrorCode::Duplicate
        | IcpRefillErrorCode::InvalidLedgerBlockIndex
        | IcpRefillErrorCode::LedgerTransferFailed
        | IcpRefillErrorCode::TransferWindowStale => "transfer",
        IcpRefillErrorCode::InvalidTransaction
        | IcpRefillErrorCode::NotifyFailed
        | IcpRefillErrorCode::NotifyMaxAttempts
        | IcpRefillErrorCode::Processing
        | IcpRefillErrorCode::Refunded
        | IcpRefillErrorCode::TransactionTooOld => "notify",
        IcpRefillErrorCode::FabricationUnavailable => "fabricate",
    }
}

const fn status_label(status: IcpRefillStatus) -> &'static str {
    match status {
        IcpRefillStatus::Completed => "completed",
        IcpRefillStatus::Failed => "failed",
        IcpRefillStatus::InvalidTransaction => "invalid_transaction",
        IcpRefillStatus::NotifyProcessing => "notify_processing",
        IcpRefillStatus::Refunded => "refunded",
        IcpRefillStatus::Requested => "requested",
        IcpRefillStatus::TransactionTooOld => "transaction_too_old",
        IcpRefillStatus::Transferred => "transferred",
    }
}

const fn error_label(error: IcpRefillErrorCode) -> &'static str {
    match error {
        IcpRefillErrorCode::BadFee => "bad_fee",
        IcpRefillErrorCode::Duplicate => "duplicate",
        IcpRefillErrorCode::FabricationUnavailable => "fabrication_unavailable",
        IcpRefillErrorCode::InvalidLedgerBlockIndex => "invalid_ledger_block_index",
        IcpRefillErrorCode::InvalidTransaction => "invalid_transaction",
        IcpRefillErrorCode::LedgerTransferFailed => "ledger_transfer_failed",
        IcpRefillErrorCode::NotifyFailed => "notify_failed",
        IcpRefillErrorCode::NotifyMaxAttempts => "notify_max_attempts",
        IcpRefillErrorCode::Processing => "processing",
        IcpRefillErrorCode::RateGateDenied => "rate_gate_denied",
        IcpRefillErrorCode::Refunded => "refunded",
        IcpRefillErrorCode::RequestDenied => "request_denied",
        IcpRefillErrorCode::TransactionTooOld => "transaction_too_old",
        IcpRefillErrorCode::TransferWindowStale => "transfer_window_stale",
    }
}
