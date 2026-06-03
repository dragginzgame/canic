#![allow(dead_code)]

use crate::{
    InternalError,
    cdk::{
        candid::Nat,
        types::{Principal, Subaccount},
    },
    dto::icp_refill::{IcpRefillErrorCode, IcpRefillResponse, IcpRefillStatus},
    ops::storage::StorageOpsError,
    storage::stable::icp_refill::{IcpRefillRecord, IcpRefillRecordKey, IcpRefillRecords},
};
use thiserror::Error as ThisError;

const ERROR_MESSAGE_MAX_CHARS: usize = 512;

///
/// IcpRefillRecordOpsError
///

#[derive(Debug, ThisError)]
pub enum IcpRefillRecordOpsError {
    #[error("ICP refill record id space exhausted")]
    IdOverflow,

    #[error("ICP refill operation id conflicts with existing record {id}")]
    OperationConflict { id: u64 },

    #[error("ICP refill record {0} not found")]
    RecordNotFound(u64),
}

impl From<IcpRefillRecordOpsError> for InternalError {
    fn from(err: IcpRefillRecordOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}

///
/// IcpRefillRecordCreateInput
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcpRefillRecordCreateInput {
    pub operation_id: [u8; 32],
    pub source_canister: Principal,
    pub source_subaccount: Option<Subaccount>,
    pub target_canister: Principal,
    pub ledger_canister_id: Principal,
    pub cmc_canister_id: Principal,
    pub cmc_to_account_owner: Principal,
    pub cmc_to_account_subaccount: Option<Subaccount>,
    pub amount_e8s: u64,
    pub fee_e8s: u64,
    pub memo: Vec<u8>,
    pub created_at_time_ns: u64,
    pub now_ns: u64,
}

///
/// IcpRefillRecordOps
///

pub struct IcpRefillRecordOps;

impl IcpRefillRecordOps {
    pub fn insert(record: IcpRefillRecord) -> Option<IcpRefillRecord> {
        IcpRefillRecords::insert(record)
    }

    #[must_use]
    pub fn get(id: u64) -> Option<IcpRefillRecord> {
        IcpRefillRecords::get(id)
    }

    #[must_use]
    pub fn entries() -> Vec<(IcpRefillRecordKey, IcpRefillRecord)> {
        IcpRefillRecords::entries(0, usize::MAX)
    }

    pub fn find_by_operation_id(operation_id: [u8; 32]) -> Option<IcpRefillRecord> {
        Self::entries()
            .into_iter()
            .map(|(_key, record)| record)
            .find(|record| record.operation_id == operation_id)
    }

    pub fn create_or_get(
        input: IcpRefillRecordCreateInput,
    ) -> Result<IcpRefillRecord, InternalError> {
        if let Some(existing) = Self::find_by_operation_id(input.operation_id) {
            ensure_compatible_operation(&existing, &input)?;
            return Ok(existing);
        }

        let id = next_id()?;
        let record = IcpRefillRecord {
            id,
            operation_id: input.operation_id,
            source_canister: input.source_canister,
            source_subaccount: input.source_subaccount,
            target_canister: input.target_canister,
            ledger_canister_id: input.ledger_canister_id,
            cmc_canister_id: input.cmc_canister_id,
            cmc_to_account_owner: input.cmc_to_account_owner,
            cmc_to_account_subaccount: input.cmc_to_account_subaccount,
            amount_e8s: input.amount_e8s,
            fee_e8s: input.fee_e8s,
            memo: input.memo,
            created_at_time_ns: input.created_at_time_ns,
            ledger_block_index: None,
            notify_attempts: 0,
            cycles_sent: None,
            status: IcpRefillStatus::Requested,
            error_code: None,
            error_message: None,
            refund_block_index: None,
            transaction_too_old_min_block_index: None,
            created_at_ns: input.now_ns,
            updated_at_ns: input.now_ns,
        };

        Self::insert(record.clone());

        Ok(record)
    }

    pub fn mark_transferred(
        id: u64,
        ledger_block_index: u64,
        now_ns: u64,
    ) -> Result<IcpRefillRecord, InternalError> {
        update_record(id, now_ns, |record| {
            record.ledger_block_index = Some(ledger_block_index);
            clear_error(record);
            record.status = IcpRefillStatus::Transferred;
        })
    }

    pub fn mark_duplicate_transferred(
        id: u64,
        ledger_block_index: u64,
        now_ns: u64,
    ) -> Result<IcpRefillRecord, InternalError> {
        update_record(id, now_ns, |record| {
            record.ledger_block_index = Some(ledger_block_index);
            set_error(
                record,
                IcpRefillStatus::Transferred,
                IcpRefillErrorCode::Duplicate,
                Some("ledger transfer duplicate; reusing duplicate block".to_string()),
            );
        })
    }

    pub fn mark_transfer_failed(
        id: u64,
        error_code: IcpRefillErrorCode,
        error_message: String,
        now_ns: u64,
    ) -> Result<IcpRefillRecord, InternalError> {
        update_record(id, now_ns, |record| {
            set_failure(record, error_code, error_message);
        })
    }

    pub fn mark_bad_fee(
        id: u64,
        expected_fee_e8s: u64,
        error_message: String,
        now_ns: u64,
    ) -> Result<IcpRefillRecord, InternalError> {
        update_record(id, now_ns, |record| {
            record.fee_e8s = expected_fee_e8s;
            set_failure(record, IcpRefillErrorCode::BadFee, error_message);
        })
    }

    pub fn mark_notify_attempt_started(
        id: u64,
        now_ns: u64,
    ) -> Result<IcpRefillRecord, InternalError> {
        update_record(id, now_ns, |record| {
            record.notify_attempts = record.notify_attempts.saturating_add(1);
        })
    }

    pub fn mark_notify_processing(id: u64, now_ns: u64) -> Result<IcpRefillRecord, InternalError> {
        update_record(id, now_ns, |record| {
            set_error(
                record,
                IcpRefillStatus::NotifyProcessing,
                IcpRefillErrorCode::Processing,
                None,
            );
        })
    }

    pub fn mark_completed(
        id: u64,
        cycles_sent: Nat,
        now_ns: u64,
    ) -> Result<IcpRefillRecord, InternalError> {
        update_record(id, now_ns, |record| {
            record.cycles_sent = Some(cycles_sent);
            clear_error(record);
            record.status = IcpRefillStatus::Completed;
        })
    }

    pub fn mark_refunded(
        id: u64,
        refund_block_index: Option<u64>,
        reason: String,
        now_ns: u64,
    ) -> Result<IcpRefillRecord, InternalError> {
        update_record(id, now_ns, |record| {
            record.refund_block_index = refund_block_index;
            set_error(
                record,
                IcpRefillStatus::Refunded,
                IcpRefillErrorCode::Refunded,
                Some(reason),
            );
        })
    }

    pub fn mark_invalid_transaction(
        id: u64,
        reason: String,
        now_ns: u64,
    ) -> Result<IcpRefillRecord, InternalError> {
        update_record(id, now_ns, |record| {
            set_error(
                record,
                IcpRefillStatus::InvalidTransaction,
                IcpRefillErrorCode::InvalidTransaction,
                Some(reason),
            );
        })
    }

    pub fn mark_transaction_too_old(
        id: u64,
        min_block_index: Option<u64>,
        now_ns: u64,
    ) -> Result<IcpRefillRecord, InternalError> {
        update_record(id, now_ns, |record| {
            record.transaction_too_old_min_block_index = min_block_index;
            set_error(
                record,
                IcpRefillStatus::TransactionTooOld,
                IcpRefillErrorCode::TransactionTooOld,
                None,
            );
        })
    }

    pub fn mark_transfer_window_stale(
        id: u64,
        now_ns: u64,
    ) -> Result<IcpRefillRecord, InternalError> {
        update_record(id, now_ns, |record| {
            set_failure(
                record,
                IcpRefillErrorCode::TransferWindowStale,
                "transfer retry window expired before ledger block was recorded".to_string(),
            );
        })
    }

    pub fn mark_notify_failed(
        id: u64,
        error_message: String,
        now_ns: u64,
    ) -> Result<IcpRefillRecord, InternalError> {
        update_record(id, now_ns, |record| {
            set_failure(record, IcpRefillErrorCode::NotifyFailed, error_message);
        })
    }

    pub fn mark_notify_max_attempts(
        id: u64,
        error_message: String,
        now_ns: u64,
    ) -> Result<IcpRefillRecord, InternalError> {
        update_record(id, now_ns, |record| {
            set_failure(record, IcpRefillErrorCode::NotifyMaxAttempts, error_message);
        })
    }

    #[must_use]
    pub fn to_response(record: &IcpRefillRecord) -> IcpRefillResponse {
        IcpRefillResponse {
            operation_id: record.operation_id,
            status: record.status,
            ledger_block_index: record.ledger_block_index,
            cycles_sent: record.cycles_sent.clone(),
            error_code: record.error_code,
            error_message: record.error_message.clone(),
        }
    }
}

fn next_id() -> Result<u64, IcpRefillRecordOpsError> {
    IcpRefillRecords::entries(0, usize::MAX)
        .into_iter()
        .map(|(key, _record)| key.0)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(IcpRefillRecordOpsError::IdOverflow)
}

fn update_record(
    id: u64,
    now_ns: u64,
    update: impl FnOnce(&mut IcpRefillRecord),
) -> Result<IcpRefillRecord, InternalError> {
    let mut record =
        IcpRefillRecordOps::get(id).ok_or(IcpRefillRecordOpsError::RecordNotFound(id))?;
    update(&mut record);
    record.updated_at_ns = now_ns;
    IcpRefillRecordOps::insert(record.clone());
    Ok(record)
}

fn ensure_compatible_operation(
    existing: &IcpRefillRecord,
    input: &IcpRefillRecordCreateInput,
) -> Result<(), IcpRefillRecordOpsError> {
    if existing.source_canister == input.source_canister
        && existing.source_subaccount == input.source_subaccount
        && existing.target_canister == input.target_canister
        && existing.ledger_canister_id == input.ledger_canister_id
        && existing.cmc_canister_id == input.cmc_canister_id
        && existing.cmc_to_account_owner == input.cmc_to_account_owner
        && existing.cmc_to_account_subaccount == input.cmc_to_account_subaccount
        && existing.amount_e8s == input.amount_e8s
        && existing.fee_e8s == input.fee_e8s
        && existing.memo == input.memo
        && existing.created_at_time_ns == input.created_at_time_ns
    {
        return Ok(());
    }

    Err(IcpRefillRecordOpsError::OperationConflict { id: existing.id })
}

fn clear_error(record: &mut IcpRefillRecord) {
    record.error_code = None;
    record.error_message = None;
}

fn set_failure(
    record: &mut IcpRefillRecord,
    error_code: IcpRefillErrorCode,
    error_message: String,
) {
    set_error(
        record,
        IcpRefillStatus::Failed,
        error_code,
        Some(error_message),
    );
}

fn set_error(
    record: &mut IcpRefillRecord,
    status: IcpRefillStatus,
    error_code: IcpRefillErrorCode,
    error_message: Option<String>,
) {
    record.status = status;
    record.error_code = Some(error_code);
    record.error_message = error_message.map(truncate_error);
}

fn truncate_error(error: String) -> String {
    error.chars().take(ERROR_MESSAGE_MAX_CHARS).collect()
}
