//! Module: ops::storage::icp_refill
//!
//! Responsibility: mutate and project durable ICP refill records.
//! Does not own: policy decisions, ledger/CMC calls, or endpoint authorization.
//! Boundary: storage ops convert stable records into workflow-facing views.

use crate::{
    InternalError,
    cdk::{
        candid::Nat,
        types::{Principal, Subaccount},
    },
    domain::icp_refill::{IcpRefillErrorCode, IcpRefillMode, IcpRefillStatus},
    dto::icp_refill::{IcpRefillRequest, IcpRefillResponse},
    ops::storage::StorageOpsError,
    storage::stable::icp_refill::{
        IcpRefillRecord, IcpRefillRecordErrorCode, IcpRefillRecordKey, IcpRefillRecordStatus,
        IcpRefillRecords,
    },
    view::icp_refill::IcpRefillOperation,
};
use thiserror::Error as ThisError;

const ERROR_MESSAGE_MAX_CHARS: usize = 512;

///
/// IcpRefillRecordOpsError
///
/// Typed storage-layer failure for durable ICP refill records.
/// Owned by storage ops and converted into internal storage errors.
///

#[derive(Debug, ThisError)]
pub enum IcpRefillRecordOpsError {
    #[error("ICP refill record id space exhausted")]
    IdOverflow,

    #[error("ICP refill operation id conflicts with existing record {id}")]
    OperationConflict { id: u64 },

    #[error("ICP refill record {0} not found")]
    RecordNotFound(u64),

    #[error(
        "ICP refill retry request does not match stored operation {field}: request={request_value}, record={record_value}"
    )]
    RetryRequestMismatch {
        field: &'static str,
        request_value: String,
        record_value: String,
    },
}

impl From<IcpRefillRecordOpsError> for InternalError {
    fn from(err: IcpRefillRecordOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}

impl From<IcpRefillStatus> for IcpRefillRecordStatus {
    fn from(status: IcpRefillStatus) -> Self {
        match status {
            IcpRefillStatus::Completed => Self::Completed,
            IcpRefillStatus::Failed => Self::Failed,
            IcpRefillStatus::InvalidTransaction => Self::InvalidTransaction,
            IcpRefillStatus::NotifyProcessing => Self::NotifyProcessing,
            IcpRefillStatus::Refunded => Self::Refunded,
            IcpRefillStatus::Requested => Self::Requested,
            IcpRefillStatus::TransactionTooOld => Self::TransactionTooOld,
            IcpRefillStatus::Transferred => Self::Transferred,
        }
    }
}

impl From<IcpRefillRecordStatus> for IcpRefillStatus {
    fn from(status: IcpRefillRecordStatus) -> Self {
        match status {
            IcpRefillRecordStatus::Completed => Self::Completed,
            IcpRefillRecordStatus::Failed => Self::Failed,
            IcpRefillRecordStatus::InvalidTransaction => Self::InvalidTransaction,
            IcpRefillRecordStatus::NotifyProcessing => Self::NotifyProcessing,
            IcpRefillRecordStatus::Refunded => Self::Refunded,
            IcpRefillRecordStatus::Requested => Self::Requested,
            IcpRefillRecordStatus::TransactionTooOld => Self::TransactionTooOld,
            IcpRefillRecordStatus::Transferred => Self::Transferred,
        }
    }
}

impl PartialEq<IcpRefillStatus> for IcpRefillRecordStatus {
    fn eq(&self, other: &IcpRefillStatus) -> bool {
        IcpRefillStatus::from(*self) == *other
    }
}

impl PartialEq<IcpRefillRecordStatus> for IcpRefillStatus {
    fn eq(&self, other: &IcpRefillRecordStatus) -> bool {
        *self == Self::from(*other)
    }
}

impl From<IcpRefillErrorCode> for IcpRefillRecordErrorCode {
    fn from(error_code: IcpRefillErrorCode) -> Self {
        match error_code {
            IcpRefillErrorCode::BadFee => Self::BadFee,
            IcpRefillErrorCode::Duplicate => Self::Duplicate,
            IcpRefillErrorCode::FabricationUnavailable => Self::FabricationUnavailable,
            IcpRefillErrorCode::InvalidLedgerBlockIndex => Self::InvalidLedgerBlockIndex,
            IcpRefillErrorCode::InvalidTransaction => Self::InvalidTransaction,
            IcpRefillErrorCode::LedgerTransferFailed => Self::LedgerTransferFailed,
            IcpRefillErrorCode::NotifyFailed => Self::NotifyFailed,
            IcpRefillErrorCode::NotifyMaxAttempts => Self::NotifyMaxAttempts,
            IcpRefillErrorCode::Processing => Self::Processing,
            IcpRefillErrorCode::RateGateDenied => Self::RateGateDenied,
            IcpRefillErrorCode::Refunded => Self::Refunded,
            IcpRefillErrorCode::RequestDenied => Self::RequestDenied,
            IcpRefillErrorCode::TransactionTooOld => Self::TransactionTooOld,
            IcpRefillErrorCode::TransferWindowStale => Self::TransferWindowStale,
        }
    }
}

impl From<IcpRefillRecordErrorCode> for IcpRefillErrorCode {
    fn from(error_code: IcpRefillRecordErrorCode) -> Self {
        match error_code {
            IcpRefillRecordErrorCode::BadFee => Self::BadFee,
            IcpRefillRecordErrorCode::Duplicate => Self::Duplicate,
            IcpRefillRecordErrorCode::FabricationUnavailable => Self::FabricationUnavailable,
            IcpRefillRecordErrorCode::InvalidLedgerBlockIndex => Self::InvalidLedgerBlockIndex,
            IcpRefillRecordErrorCode::InvalidTransaction => Self::InvalidTransaction,
            IcpRefillRecordErrorCode::LedgerTransferFailed => Self::LedgerTransferFailed,
            IcpRefillRecordErrorCode::NotifyFailed => Self::NotifyFailed,
            IcpRefillRecordErrorCode::NotifyMaxAttempts => Self::NotifyMaxAttempts,
            IcpRefillRecordErrorCode::Processing => Self::Processing,
            IcpRefillRecordErrorCode::RateGateDenied => Self::RateGateDenied,
            IcpRefillRecordErrorCode::Refunded => Self::Refunded,
            IcpRefillRecordErrorCode::RequestDenied => Self::RequestDenied,
            IcpRefillRecordErrorCode::TransactionTooOld => Self::TransactionTooOld,
            IcpRefillRecordErrorCode::TransferWindowStale => Self::TransferWindowStale,
        }
    }
}

impl PartialEq<IcpRefillErrorCode> for IcpRefillRecordErrorCode {
    fn eq(&self, other: &IcpRefillErrorCode) -> bool {
        IcpRefillErrorCode::from(*self) == *other
    }
}

impl PartialEq<IcpRefillRecordErrorCode> for IcpRefillErrorCode {
    fn eq(&self, other: &IcpRefillRecordErrorCode) -> bool {
        *self == Self::from(*other)
    }
}

///
/// IcpRefillRecordCreateInput
///
/// Storage input for creating a durable ICP refill record.
/// Owned by storage ops and supplied by workflow orchestration after policy checks.
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
/// IcpRefillOperationCreateInput
///
/// Workflow-facing input for creating or resuming an ICP refill operation.
/// Owned by storage ops and converted into stable record creation input.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcpRefillOperationCreateInput {
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

impl From<IcpRefillOperationCreateInput> for IcpRefillRecordCreateInput {
    fn from(input: IcpRefillOperationCreateInput) -> Self {
        Self {
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
            now_ns: input.now_ns,
        }
    }
}

///
/// IcpRefillStoreOps
///
/// Workflow-facing facade over durable ICP refill storage records.
/// Owned by storage ops and consumed by ICP refill workflows.
///

pub struct IcpRefillStoreOps;

impl IcpRefillStoreOps {
    pub fn find_by_operation_id(operation_id: [u8; 32]) -> Option<IcpRefillOperation> {
        IcpRefillRecordOps::find_by_operation_id(operation_id).map(record_to_operation)
    }

    pub fn validate_retry_request_matches_operation(
        request: &IcpRefillRequest,
        operation: &IcpRefillOperation,
    ) -> Result<(), InternalError> {
        ensure_retry_field(
            "source_canister",
            request.source_canister,
            operation.source_canister,
        )?;
        ensure_retry_field(
            "source_subaccount",
            request.source_subaccount,
            operation.source_subaccount,
        )?;
        ensure_retry_field(
            "target_canister",
            request.target_canister,
            operation.target_canister,
        )?;
        ensure_retry_field("amount_e8s", request.amount_e8s, operation.amount_e8s)?;

        Ok(())
    }

    pub fn has_in_flight_for_key(
        source_canister: Principal,
        source_subaccount: Option<Subaccount>,
        target_canister: Principal,
        except_operation_id: [u8; 32],
    ) -> bool {
        IcpRefillRecordOps::has_in_flight_for_key(
            source_canister,
            source_subaccount,
            target_canister,
            except_operation_id,
        )
    }

    pub fn find_resumable_hub_self_refill(self_pid: Principal) -> Option<IcpRefillOperation> {
        IcpRefillRecordOps::find_resumable_hub_self_refill(self_pid).map(record_to_operation)
    }

    #[must_use]
    pub const fn is_in_flight_status(status: IcpRefillStatus) -> bool {
        IcpRefillRecordOps::is_in_flight_status(status)
    }

    #[must_use]
    pub const fn is_resumable(operation: &IcpRefillOperation) -> bool {
        Self::is_in_flight_status(operation.status)
            || Self::can_retry_notify(operation)
            || Self::can_retry_bad_fee(operation)
    }

    #[must_use]
    pub const fn can_retry_notify(operation: &IcpRefillOperation) -> bool {
        operation.ledger_block_index.is_some()
            && matches!(operation.status, IcpRefillStatus::Failed)
            && matches!(operation.error_code, Some(IcpRefillErrorCode::NotifyFailed))
    }

    #[must_use]
    pub const fn can_retry_bad_fee(operation: &IcpRefillOperation) -> bool {
        operation.ledger_block_index.is_none()
            && matches!(operation.status, IcpRefillStatus::Failed)
            && matches!(operation.error_code, Some(IcpRefillErrorCode::BadFee))
    }

    #[must_use]
    pub const fn should_notify(operation: &IcpRefillOperation) -> bool {
        operation.ledger_block_index.is_some()
            && (matches!(
                operation.status,
                IcpRefillStatus::Transferred | IcpRefillStatus::NotifyProcessing
            ) || Self::can_retry_notify(operation))
    }

    #[must_use]
    pub const fn transfer_window_stale(
        operation: &IcpRefillOperation,
        now_ns: u64,
        retry_window_nanos: u64,
    ) -> bool {
        operation.ledger_block_index.is_none()
            && (matches!(operation.status, IcpRefillStatus::Requested)
                || Self::can_retry_bad_fee(operation))
            && operation
                .created_at_time_ns
                .saturating_add(retry_window_nanos)
                < now_ns
    }

    pub fn create_or_get(
        input: IcpRefillOperationCreateInput,
    ) -> Result<IcpRefillOperation, InternalError> {
        IcpRefillRecordOps::create_or_get(input.into()).map(record_to_operation)
    }

    pub fn mark_transferred(
        id: u64,
        ledger_block_index: u64,
        now_ns: u64,
    ) -> Result<IcpRefillOperation, InternalError> {
        IcpRefillRecordOps::mark_transferred(id, ledger_block_index, now_ns)
            .map(record_to_operation)
    }

    pub fn mark_duplicate_transferred(
        id: u64,
        ledger_block_index: u64,
        now_ns: u64,
    ) -> Result<IcpRefillOperation, InternalError> {
        IcpRefillRecordOps::mark_duplicate_transferred(id, ledger_block_index, now_ns)
            .map(record_to_operation)
    }

    pub fn mark_transfer_failed(
        id: u64,
        error_code: IcpRefillErrorCode,
        error_message: String,
        now_ns: u64,
    ) -> Result<IcpRefillOperation, InternalError> {
        IcpRefillRecordOps::mark_transfer_failed(id, error_code, error_message, now_ns)
            .map(record_to_operation)
    }

    pub fn mark_bad_fee(
        id: u64,
        expected_fee_e8s: u64,
        error_message: String,
        now_ns: u64,
    ) -> Result<IcpRefillOperation, InternalError> {
        IcpRefillRecordOps::mark_bad_fee(id, expected_fee_e8s, error_message, now_ns)
            .map(record_to_operation)
    }

    pub fn mark_notify_attempt_started(
        id: u64,
        now_ns: u64,
    ) -> Result<IcpRefillOperation, InternalError> {
        IcpRefillRecordOps::mark_notify_attempt_started(id, now_ns).map(record_to_operation)
    }

    pub fn mark_notify_processing(
        id: u64,
        now_ns: u64,
    ) -> Result<IcpRefillOperation, InternalError> {
        IcpRefillRecordOps::mark_notify_processing(id, now_ns).map(record_to_operation)
    }

    pub fn mark_completed(
        id: u64,
        cycles_sent: Nat,
        now_ns: u64,
    ) -> Result<IcpRefillOperation, InternalError> {
        IcpRefillRecordOps::mark_completed(id, cycles_sent, now_ns).map(record_to_operation)
    }

    pub fn mark_refunded(
        id: u64,
        refund_block_index: Option<u64>,
        reason: String,
        now_ns: u64,
    ) -> Result<IcpRefillOperation, InternalError> {
        IcpRefillRecordOps::mark_refunded(id, refund_block_index, reason, now_ns)
            .map(record_to_operation)
    }

    pub fn mark_invalid_transaction(
        id: u64,
        reason: String,
        now_ns: u64,
    ) -> Result<IcpRefillOperation, InternalError> {
        IcpRefillRecordOps::mark_invalid_transaction(id, reason, now_ns).map(record_to_operation)
    }

    pub fn mark_transaction_too_old(
        id: u64,
        min_block_index: Option<u64>,
        now_ns: u64,
    ) -> Result<IcpRefillOperation, InternalError> {
        IcpRefillRecordOps::mark_transaction_too_old(id, min_block_index, now_ns)
            .map(record_to_operation)
    }

    pub fn mark_transfer_window_stale(
        id: u64,
        now_ns: u64,
    ) -> Result<IcpRefillOperation, InternalError> {
        IcpRefillRecordOps::mark_transfer_window_stale(id, now_ns).map(record_to_operation)
    }

    pub fn mark_notify_failed(
        id: u64,
        error_message: String,
        now_ns: u64,
    ) -> Result<IcpRefillOperation, InternalError> {
        IcpRefillRecordOps::mark_notify_failed(id, error_message, now_ns).map(record_to_operation)
    }

    pub fn mark_notify_max_attempts(
        id: u64,
        error_message: String,
        now_ns: u64,
    ) -> Result<IcpRefillOperation, InternalError> {
        IcpRefillRecordOps::mark_notify_max_attempts(id, error_message, now_ns)
            .map(record_to_operation)
    }

    #[must_use]
    pub fn nat_to_u128_saturating(value: &Nat) -> u128 {
        IcpRefillRecordOps::nat_to_u128_saturating(value)
    }

    #[must_use]
    pub const fn to_request(operation: &IcpRefillOperation) -> IcpRefillRequest {
        IcpRefillRequest {
            operation_id: operation.operation_id,
            source_canister: operation.source_canister,
            source_subaccount: operation.source_subaccount,
            target_canister: operation.target_canister,
            amount_e8s: operation.amount_e8s,
            dry_run: false,
            mode: IcpRefillMode::Canister,
        }
    }

    #[must_use]
    pub fn to_response(operation: &IcpRefillOperation) -> IcpRefillResponse {
        IcpRefillResponse {
            operation_id: operation.operation_id,
            status: operation.status,
            ledger_block_index: operation.ledger_block_index,
            cycles_sent: operation.cycles_sent.clone(),
            error_code: operation.error_code,
            error_message: operation.error_message.clone(),
        }
    }
}

///
/// IcpRefillRecordOps
///
/// Low-level deterministic operations over stable ICP refill records.
/// Owned by storage ops and used by metrics and workflow storage facades.
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

    #[must_use]
    pub fn records() -> Vec<IcpRefillRecord> {
        Self::entries()
            .into_iter()
            .map(|(_key, record)| record)
            .collect()
    }

    #[must_use]
    pub fn nat_to_u128_saturating(value: &Nat) -> u128 {
        u128::try_from(value.0.clone()).unwrap_or(u128::MAX)
    }

    pub fn find_by_operation_id(operation_id: [u8; 32]) -> Option<IcpRefillRecord> {
        Self::records()
            .into_iter()
            .find(|record| record.operation_id == operation_id)
    }

    pub fn has_in_flight_for_key(
        source_canister: Principal,
        source_subaccount: Option<Subaccount>,
        target_canister: Principal,
        except_operation_id: [u8; 32],
    ) -> bool {
        Self::records().into_iter().any(|record| {
            record.source_canister == source_canister
                && record.source_subaccount == source_subaccount
                && record.target_canister == target_canister
                && record_status_is_in_flight(record.status)
                && record.operation_id != except_operation_id
        })
    }

    pub fn find_resumable_hub_self_refill(self_pid: Principal) -> Option<IcpRefillRecord> {
        Self::records().into_iter().find(|record| {
            record.source_canister == self_pid
                && record.source_subaccount.is_none()
                && record.target_canister == self_pid
                && Self::is_resumable(record)
        })
    }

    #[must_use]
    pub const fn is_in_flight_status(status: IcpRefillStatus) -> bool {
        matches!(
            status,
            IcpRefillStatus::Requested
                | IcpRefillStatus::Transferred
                | IcpRefillStatus::NotifyProcessing
        )
    }

    #[must_use]
    pub const fn is_resumable(record: &IcpRefillRecord) -> bool {
        record_status_is_in_flight(record.status)
            || Self::can_retry_notify(record)
            || Self::can_retry_bad_fee(record)
    }

    #[must_use]
    pub const fn can_retry_notify(record: &IcpRefillRecord) -> bool {
        record.ledger_block_index.is_some()
            && matches!(record.status, IcpRefillRecordStatus::Failed)
            && matches!(
                record.error_code,
                Some(IcpRefillRecordErrorCode::NotifyFailed)
            )
    }

    #[must_use]
    pub const fn can_retry_bad_fee(record: &IcpRefillRecord) -> bool {
        record.ledger_block_index.is_none()
            && matches!(record.status, IcpRefillRecordStatus::Failed)
            && matches!(record.error_code, Some(IcpRefillRecordErrorCode::BadFee))
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
            status: IcpRefillRecordStatus::Requested,
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
            record.status = IcpRefillRecordStatus::Transferred;
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
                IcpRefillRecordStatus::Transferred,
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
                IcpRefillRecordStatus::NotifyProcessing,
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
            record.status = IcpRefillRecordStatus::Completed;
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
                IcpRefillRecordStatus::Refunded,
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
                IcpRefillRecordStatus::InvalidTransaction,
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
                IcpRefillRecordStatus::TransactionTooOld,
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

fn ensure_retry_field<T>(
    field: &'static str,
    request_value: T,
    record_value: T,
) -> Result<(), InternalError>
where
    T: Eq + std::fmt::Debug,
{
    if request_value == record_value {
        return Ok(());
    }

    Err(IcpRefillRecordOpsError::RetryRequestMismatch {
        field,
        request_value: format!("{request_value:?}"),
        record_value: format!("{record_value:?}"),
    }
    .into())
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
        IcpRefillRecordStatus::Failed,
        error_code,
        Some(error_message),
    );
}

fn set_error(
    record: &mut IcpRefillRecord,
    status: IcpRefillRecordStatus,
    error_code: IcpRefillErrorCode,
    error_message: Option<String>,
) {
    record.status = status;
    record.error_code = Some(error_code.into());
    record.error_message = error_message.map(truncate_error);
}

fn truncate_error(error: String) -> String {
    error.chars().take(ERROR_MESSAGE_MAX_CHARS).collect()
}

const fn record_status_is_in_flight(status: IcpRefillRecordStatus) -> bool {
    matches!(
        status,
        IcpRefillRecordStatus::Requested
            | IcpRefillRecordStatus::Transferred
            | IcpRefillRecordStatus::NotifyProcessing
    )
}

fn record_to_operation(record: IcpRefillRecord) -> IcpRefillOperation {
    IcpRefillOperation {
        id: record.id,
        operation_id: record.operation_id,
        source_canister: record.source_canister,
        source_subaccount: record.source_subaccount,
        target_canister: record.target_canister,
        ledger_canister_id: record.ledger_canister_id,
        cmc_canister_id: record.cmc_canister_id,
        amount_e8s: record.amount_e8s,
        fee_e8s: record.fee_e8s,
        memo: record.memo,
        created_at_time_ns: record.created_at_time_ns,
        ledger_block_index: record.ledger_block_index,
        notify_attempts: record.notify_attempts,
        cycles_sent: record.cycles_sent,
        status: record.status.into(),
        error_code: record.error_code.map(Into::into),
        error_message: record.error_message,
    }
}
