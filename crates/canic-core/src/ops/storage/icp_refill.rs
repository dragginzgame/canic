//! Module: ops::storage::icp_refill
//!
//! Responsibility: mutate and project durable ICP refill records.
//! Does not own: policy decisions, ledger/CMC calls, or endpoint authorization.
//! Boundary: storage ops convert stable records into workflow-facing views.

use crate::{
    InternalError,
    cdk::{
        candid::Nat,
        types::{Cycles, Principal, Subaccount},
    },
    domain::icp_refill::{IcpRefillErrorCode, IcpRefillStatus, icp_refill_outcome_is_resumable},
    dto::icp_refill::{IcpRefillRequest, IcpRefillResponse},
    ops::storage::StorageOpsError,
    storage::stable::icp_refill::{
        IcpRefillRecord, IcpRefillRecordErrorCode, IcpRefillRecordStatus, IcpRefillRecords,
    },
    view::icp_refill::IcpRefillOperation,
};
use std::{cell::RefCell, collections::BTreeMap};
use thiserror::Error as ThisError;

const ERROR_MESSAGE_MAX_CHARS: usize = 512;

thread_local! {
    static ICP_REFILL_DERIVED_INDEX: RefCell<IcpRefillDerivedIndex> =
        RefCell::new(IcpRefillDerivedIndex::default());
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct IcpRefillActiveKey {
    source_canister: Principal,
    source_subaccount: Option<Subaccount>,
    target_canister: Principal,
}

impl IcpRefillActiveKey {
    const fn from_record(record: &IcpRefillRecord) -> Self {
        Self {
            source_canister: record.source_canister,
            source_subaccount: record.source_subaccount,
            target_canister: record.target_canister,
        }
    }

    const fn new(
        source_canister: Principal,
        source_subaccount: Option<Subaccount>,
        target_canister: Principal,
    ) -> Self {
        Self {
            source_canister,
            source_subaccount,
            target_canister,
        }
    }
}

///
/// IcpRefillMetricStatusCount
///
/// Derived count for one persisted ICP-refill status and error combination.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcpRefillMetricStatusCount {
    pub status: IcpRefillRecordStatus,
    pub error_code: Option<IcpRefillRecordErrorCode>,
    pub count: u64,
}

///
/// IcpRefillMetricErrorCount
///
/// Derived count for one persisted ICP-refill error code.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcpRefillMetricErrorCount {
    pub error_code: IcpRefillRecordErrorCode,
    pub count: u64,
}

///
/// IcpRefillMetricTargetTotal
///
/// Derived cumulative amount and completed-cycle totals for one refill target.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcpRefillMetricTargetTotal {
    pub target_canister: Principal,
    pub amount_e8s: u128,
    pub cycles_sent: Option<u128>,
}

///
/// IcpRefillMetricSnapshot
///
/// Bounded-by-dimension metric projection maintained with refill record writes.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IcpRefillMetricSnapshot {
    pub statuses: Vec<IcpRefillMetricStatusCount>,
    pub errors: Vec<IcpRefillMetricErrorCount>,
    pub targets: Vec<IcpRefillMetricTargetTotal>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct IcpRefillTargetTotals {
    amount_e8s: u128,
    cycles_sent: u128,
    completed_cycles_records: u64,
}

#[derive(Default)]
struct IcpRefillDerivedIndex {
    operation_ids: BTreeMap<[u8; 32], u64>,
    active_operations: BTreeMap<IcpRefillActiveKey, u64>,
    status_counts: BTreeMap<(IcpRefillRecordStatus, Option<IcpRefillRecordErrorCode>), u64>,
    error_counts: BTreeMap<IcpRefillRecordErrorCode, u64>,
    target_totals: BTreeMap<Principal, IcpRefillTargetTotals>,
    max_id: u64,
}

///
/// IcpRefillRecordOpsError
///
/// Typed storage-layer failure for durable ICP refill records.
/// Owned by storage ops and converted into internal storage errors.
///

#[derive(Debug, ThisError)]
pub enum IcpRefillRecordOpsError {
    #[error("ICP refill conflicts with active record {id} for the same source and target")]
    ConcurrentOperation { id: u64 },

    #[error("completed ICP refill record {id} cycles_sent does not fit in u128: {value}")]
    CyclesSentOverflow { id: u64, value: Nat },

    #[error(
        "ICP refill active-operation index conflicts between records {existing_id} and {conflicting_id}"
    )]
    DuplicateActiveIndex {
        existing_id: u64,
        conflicting_id: u64,
    },

    #[error(
        "ICP refill operation index conflicts between records {existing_id} and {conflicting_id}"
    )]
    DuplicateOperationIndex {
        existing_id: u64,
        conflicting_id: u64,
    },

    #[error("ICP refill record id space exhausted")]
    IdOverflow,

    #[error("ICP refill {index} index points to missing record {id}")]
    IndexRecordMissing { index: &'static str, id: u64 },

    #[error("ICP refill {index} index does not match record {id}")]
    IndexRecordMismatch { index: &'static str, id: u64 },

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
            IcpRefillErrorCode::CyclesSentOverflow => Self::CyclesSentOverflow,
            IcpRefillErrorCode::Duplicate => Self::Duplicate,
            IcpRefillErrorCode::InvalidLedgerBlockIndex => Self::InvalidLedgerBlockIndex,
            IcpRefillErrorCode::InvalidTransaction => Self::InvalidTransaction,
            IcpRefillErrorCode::LedgerTransferFailed => Self::LedgerTransferFailed,
            IcpRefillErrorCode::NotifyFailed => Self::NotifyFailed,
            IcpRefillErrorCode::NotifyMaxAttempts => Self::NotifyMaxAttempts,
            IcpRefillErrorCode::Processing => Self::Processing,
            IcpRefillErrorCode::Refunded => Self::Refunded,
            IcpRefillErrorCode::TransactionTooOld => Self::TransactionTooOld,
            IcpRefillErrorCode::TransferWindowStale => Self::TransferWindowStale,
        }
    }
}

impl From<IcpRefillRecordErrorCode> for IcpRefillErrorCode {
    fn from(error_code: IcpRefillRecordErrorCode) -> Self {
        match error_code {
            IcpRefillRecordErrorCode::BadFee => Self::BadFee,
            IcpRefillRecordErrorCode::CyclesSentOverflow => Self::CyclesSentOverflow,
            IcpRefillRecordErrorCode::Duplicate => Self::Duplicate,
            IcpRefillRecordErrorCode::InvalidLedgerBlockIndex => Self::InvalidLedgerBlockIndex,
            IcpRefillRecordErrorCode::InvalidTransaction => Self::InvalidTransaction,
            IcpRefillRecordErrorCode::LedgerTransferFailed => Self::LedgerTransferFailed,
            IcpRefillRecordErrorCode::NotifyFailed => Self::NotifyFailed,
            IcpRefillRecordErrorCode::NotifyMaxAttempts => Self::NotifyMaxAttempts,
            IcpRefillRecordErrorCode::Processing => Self::Processing,
            IcpRefillRecordErrorCode::Refunded => Self::Refunded,
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
    /// Rebuild all heap-only lookup and metric indexes from canonical records.
    pub fn rebuild_indexes() -> Result<(), InternalError> {
        IcpRefillRecordOps::rebuild_indexes()?;
        Ok(())
    }

    /// Return the number of refill operations that still require the current
    /// binary's recovery contract.
    #[must_use]
    pub fn resumable_operation_count() -> usize {
        IcpRefillRecordOps::resumable_operation_count()
    }

    pub fn find_by_operation_id(
        operation_id: [u8; 32],
    ) -> Result<Option<IcpRefillOperation>, InternalError> {
        Ok(IcpRefillRecordOps::find_by_operation_id(operation_id)?.map(record_to_operation))
    }

    pub fn validate_retry_request_matches_operation(
        request: &IcpRefillRequest,
        root_canister: Principal,
        operation: &IcpRefillOperation,
    ) -> Result<(), InternalError> {
        ensure_retry_field("source_canister", root_canister, operation.source_canister)?;
        ensure_retry_field(
            "source_subaccount",
            request.source_subaccount,
            operation.source_subaccount,
        )?;
        ensure_retry_field("target_canister", root_canister, operation.target_canister)?;
        ensure_retry_field("amount_e8s", request.amount_e8s, operation.amount_e8s)?;

        Ok(())
    }

    pub fn has_active_for_key(
        source_canister: Principal,
        source_subaccount: Option<Subaccount>,
        target_canister: Principal,
        except_operation_id: [u8; 32],
    ) -> Result<bool, InternalError> {
        Ok(IcpRefillRecordOps::has_active_for_key(
            source_canister,
            source_subaccount,
            target_canister,
            except_operation_id,
        )?)
    }

    #[must_use]
    pub const fn is_resumable(operation: &IcpRefillOperation) -> bool {
        icp_refill_outcome_is_resumable(
            operation.status,
            operation.error_code,
            operation.ledger_block_index.is_some(),
        )
    }

    #[must_use]
    pub const fn can_retry_notify(operation: &IcpRefillOperation) -> bool {
        icp_refill_outcome_is_resumable(
            operation.status,
            operation.error_code,
            operation.ledger_block_index.is_some(),
        ) && matches!(operation.error_code, Some(IcpRefillErrorCode::NotifyFailed))
    }

    #[must_use]
    pub const fn can_retry_bad_fee(operation: &IcpRefillOperation) -> bool {
        icp_refill_outcome_is_resumable(
            operation.status,
            operation.error_code,
            operation.ledger_block_index.is_some(),
        ) && matches!(operation.error_code, Some(IcpRefillErrorCode::BadFee))
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
    ) -> Result<IcpRefillOperation, IcpRefillRecordOpsError> {
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

    pub fn complete_from_notified_cycles(
        id: u64,
        cycles_sent: Nat,
        now_ns: u64,
    ) -> Result<IcpRefillOperation, InternalError> {
        match Cycles::try_from(cycles_sent) {
            Ok(cycles_sent) => {
                let cycles_sent = cycles_sent.to_u128();
                IcpRefillRecordOps::mark_completed(id, cycles_sent, now_ns).map(record_to_operation)
            }
            Err(err) => IcpRefillRecordOps::mark_cycles_sent_overflow(id, err.to_string(), now_ns)
                .map(record_to_operation),
        }
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

impl IcpRefillDerivedIndex {
    fn from_records(
        records: impl IntoIterator<Item = IcpRefillRecord>,
    ) -> Result<Self, IcpRefillRecordOpsError> {
        let mut index = Self::default();
        for record in records {
            index.add_record(&record)?;
        }
        Ok(index)
    }

    fn replace_record(
        &mut self,
        previous: Option<&IcpRefillRecord>,
        record: &IcpRefillRecord,
    ) -> Result<(), IcpRefillRecordOpsError> {
        self.validate_record(record)?;
        let previous_cycles_sent = previous.map(completed_cycles_sent).transpose()?;
        let cycles_sent = completed_cycles_sent(record)?;
        if let (Some(previous), Some(previous_cycles_sent)) = (previous, previous_cycles_sent) {
            self.remove_record(previous, previous_cycles_sent);
        }
        self.add_record_unchecked(record, cycles_sent);
        Ok(())
    }

    fn add_record(&mut self, record: &IcpRefillRecord) -> Result<(), IcpRefillRecordOpsError> {
        self.validate_record(record)?;
        let cycles_sent = completed_cycles_sent(record)?;
        self.add_record_unchecked(record, cycles_sent);
        Ok(())
    }

    fn validate_record(&self, record: &IcpRefillRecord) -> Result<(), IcpRefillRecordOpsError> {
        if let Some(existing_id) = self.operation_ids.get(&record.operation_id)
            && *existing_id != record.id
        {
            return Err(IcpRefillRecordOpsError::DuplicateOperationIndex {
                existing_id: *existing_id,
                conflicting_id: record.id,
            });
        }

        let active_key = IcpRefillActiveKey::from_record(record);
        if record_is_resumable(record)
            && let Some(existing_id) = self.active_operations.get(&active_key)
            && *existing_id != record.id
        {
            return Err(IcpRefillRecordOpsError::DuplicateActiveIndex {
                existing_id: *existing_id,
                conflicting_id: record.id,
            });
        }

        Ok(())
    }

    fn add_record_unchecked(&mut self, record: &IcpRefillRecord, cycles_sent: u128) {
        let active_key = IcpRefillActiveKey::from_record(record);
        self.operation_ids.insert(record.operation_id, record.id);
        if record_is_resumable(record) {
            self.active_operations.insert(active_key, record.id);
        }
        let status_count = self
            .status_counts
            .entry((record.status, record.error_code))
            .or_default();
        *status_count = status_count.saturating_add(1);
        if let Some(error_code) = record.error_code {
            let count = self.error_counts.entry(error_code).or_default();
            *count = count.saturating_add(1);
        }
        let totals = self
            .target_totals
            .entry(record.target_canister)
            .or_default();
        totals.amount_e8s = totals
            .amount_e8s
            .saturating_add(u128::from(record.amount_e8s));
        totals.cycles_sent = totals.cycles_sent.saturating_add(cycles_sent);
        if record.status == IcpRefillRecordStatus::Completed && record.cycles_sent.is_some() {
            totals.completed_cycles_records = totals.completed_cycles_records.saturating_add(1);
        }
        self.max_id = self.max_id.max(record.id);
    }

    fn remove_record(&mut self, record: &IcpRefillRecord, cycles_sent: u128) {
        if self.operation_ids.get(&record.operation_id) == Some(&record.id) {
            self.operation_ids.remove(&record.operation_id);
        }
        let active_key = IcpRefillActiveKey::from_record(record);
        if self.active_operations.get(&active_key) == Some(&record.id) {
            self.active_operations.remove(&active_key);
        }
        decrement_count(&mut self.status_counts, &(record.status, record.error_code));
        if let Some(error_code) = record.error_code {
            decrement_count(&mut self.error_counts, &error_code);
        }
        if let Some(totals) = self.target_totals.get_mut(&record.target_canister) {
            totals.amount_e8s = totals
                .amount_e8s
                .saturating_sub(u128::from(record.amount_e8s));
            totals.cycles_sent = totals.cycles_sent.saturating_sub(cycles_sent);
            if record.status == IcpRefillRecordStatus::Completed && record.cycles_sent.is_some() {
                totals.completed_cycles_records = totals.completed_cycles_records.saturating_sub(1);
            }
            if *totals == IcpRefillTargetTotals::default() {
                self.target_totals.remove(&record.target_canister);
            }
        }
    }

    fn metric_snapshot(&self) -> IcpRefillMetricSnapshot {
        IcpRefillMetricSnapshot {
            statuses: self
                .status_counts
                .iter()
                .map(|((status, error_code), count)| IcpRefillMetricStatusCount {
                    status: *status,
                    error_code: *error_code,
                    count: *count,
                })
                .collect(),
            errors: self
                .error_counts
                .iter()
                .map(|(error_code, count)| IcpRefillMetricErrorCount {
                    error_code: *error_code,
                    count: *count,
                })
                .collect(),
            targets: self
                .target_totals
                .iter()
                .map(|(target_canister, totals)| IcpRefillMetricTargetTotal {
                    target_canister: *target_canister,
                    amount_e8s: totals.amount_e8s,
                    cycles_sent: (totals.completed_cycles_records > 0)
                        .then_some(totals.cycles_sent),
                })
                .collect(),
        }
    }
}

fn decrement_count<K: Ord>(counts: &mut BTreeMap<K, u64>, key: &K) {
    let remove = counts.get_mut(key).is_some_and(|count| {
        *count = count.saturating_sub(1);
        *count == 0
    });
    if remove {
        counts.remove(key);
    }
}

fn record_is_resumable(record: &IcpRefillRecord) -> bool {
    icp_refill_outcome_is_resumable(
        record.status.into(),
        record.error_code.map(Into::into),
        record.ledger_block_index.is_some(),
    )
}

fn completed_cycles_sent(record: &IcpRefillRecord) -> Result<u128, IcpRefillRecordOpsError> {
    if record.status != IcpRefillRecordStatus::Completed {
        return Ok(0);
    }
    let Some(cycles_sent) = record.cycles_sent.as_ref() else {
        return Ok(0);
    };
    Cycles::try_from(cycles_sent.clone())
        .map(|cycles| cycles.to_u128())
        .map_err(|_| IcpRefillRecordOpsError::CyclesSentOverflow {
            id: record.id,
            value: cycles_sent.clone(),
        })
}

///
/// IcpRefillRecordOps
///
/// Low-level deterministic operations over stable ICP refill records.
/// Owned by storage ops and used by metrics and workflow storage facades.
///

pub struct IcpRefillRecordOps;

impl IcpRefillRecordOps {
    pub fn rebuild_indexes() -> Result<(), IcpRefillRecordOpsError> {
        let records = IcpRefillRecords::data(0, usize::MAX)
            .entries
            .into_iter()
            .map(|entry| entry.record);
        let rebuilt = IcpRefillDerivedIndex::from_records(records)?;
        ICP_REFILL_DERIVED_INDEX.with_borrow_mut(|index| *index = rebuilt);
        Ok(())
    }

    pub fn insert(
        record: IcpRefillRecord,
    ) -> Result<Option<IcpRefillRecord>, IcpRefillRecordOpsError> {
        let previous = IcpRefillRecords::get(record.id);
        ICP_REFILL_DERIVED_INDEX
            .with_borrow_mut(|index| index.replace_record(previous.as_ref(), &record))?;
        Ok(IcpRefillRecords::insert(record))
    }

    #[must_use]
    pub fn get(id: u64) -> Option<IcpRefillRecord> {
        IcpRefillRecords::get(id)
    }

    pub fn metric_snapshot() -> IcpRefillMetricSnapshot {
        ICP_REFILL_DERIVED_INDEX.with_borrow(IcpRefillDerivedIndex::metric_snapshot)
    }

    #[must_use]
    pub fn resumable_operation_count() -> usize {
        ICP_REFILL_DERIVED_INDEX.with_borrow(|index| index.active_operations.len())
    }

    pub fn find_by_operation_id(
        operation_id: [u8; 32],
    ) -> Result<Option<IcpRefillRecord>, IcpRefillRecordOpsError> {
        let id = ICP_REFILL_DERIVED_INDEX
            .with_borrow(|index| index.operation_ids.get(&operation_id).copied());
        let Some(id) = id else {
            return Ok(None);
        };
        let record = indexed_record("operation", id)?;
        if record.operation_id != operation_id {
            return Err(IcpRefillRecordOpsError::IndexRecordMismatch {
                index: "operation",
                id,
            });
        }
        Ok(Some(record))
    }

    pub fn has_active_for_key(
        source_canister: Principal,
        source_subaccount: Option<Subaccount>,
        target_canister: Principal,
        except_operation_id: [u8; 32],
    ) -> Result<bool, IcpRefillRecordOpsError> {
        Ok(Self::find_active_for_key(
            source_canister,
            source_subaccount,
            target_canister,
            Some(except_operation_id),
        )?
        .is_some())
    }

    #[must_use]
    pub fn is_resumable(record: &IcpRefillRecord) -> bool {
        record_is_resumable(record)
    }

    pub fn create_or_get(
        input: IcpRefillRecordCreateInput,
    ) -> Result<IcpRefillRecord, IcpRefillRecordOpsError> {
        if let Some(existing) = Self::find_by_operation_id(input.operation_id)? {
            ensure_compatible_operation(&existing, &input)?;
            return Ok(existing);
        }

        if let Some(existing) = Self::find_active_for_key(
            input.source_canister,
            input.source_subaccount,
            input.target_canister,
            Some(input.operation_id),
        )? {
            return Err(IcpRefillRecordOpsError::ConcurrentOperation { id: existing.id });
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

        Self::insert(record.clone())?;

        Ok(record)
    }

    fn find_active_for_key(
        source_canister: Principal,
        source_subaccount: Option<Subaccount>,
        target_canister: Principal,
        except_operation_id: Option<[u8; 32]>,
    ) -> Result<Option<IcpRefillRecord>, IcpRefillRecordOpsError> {
        let key = IcpRefillActiveKey::new(source_canister, source_subaccount, target_canister);
        let id = ICP_REFILL_DERIVED_INDEX
            .with_borrow(|index| index.active_operations.get(&key).copied());
        let Some(id) = id else {
            return Ok(None);
        };
        let record = indexed_record("active operation", id)?;
        if IcpRefillActiveKey::from_record(&record) != key || !Self::is_resumable(&record) {
            return Err(IcpRefillRecordOpsError::IndexRecordMismatch {
                index: "active operation",
                id,
            });
        }
        if except_operation_id == Some(record.operation_id) {
            return Ok(None);
        }
        Ok(Some(record))
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
        cycles_sent: u128,
        now_ns: u64,
    ) -> Result<IcpRefillRecord, InternalError> {
        update_record(id, now_ns, |record| {
            record.cycles_sent = Some(Nat::from(cycles_sent));
            clear_error(record);
            record.status = IcpRefillRecordStatus::Completed;
        })
    }

    pub fn mark_cycles_sent_overflow(
        id: u64,
        error_message: String,
        now_ns: u64,
    ) -> Result<IcpRefillRecord, InternalError> {
        update_record(id, now_ns, |record| {
            set_failure(
                record,
                IcpRefillErrorCode::CyclesSentOverflow,
                error_message,
            );
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
    ICP_REFILL_DERIVED_INDEX
        .with_borrow(|index| index.max_id)
        .checked_add(1)
        .ok_or(IcpRefillRecordOpsError::IdOverflow)
}

fn indexed_record(
    index: &'static str,
    id: u64,
) -> Result<IcpRefillRecord, IcpRefillRecordOpsError> {
    IcpRefillRecords::get(id).ok_or(IcpRefillRecordOpsError::IndexRecordMissing { index, id })
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
    IcpRefillRecordOps::insert(record.clone())?;
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::seams;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn record(id: u64, operation_id: [u8; 32], target: Principal) -> IcpRefillRecord {
        IcpRefillRecord {
            id,
            operation_id,
            source_canister: p(u8::try_from(id).expect("test id fits")),
            source_subaccount: None,
            target_canister: target,
            ledger_canister_id: p(20),
            cmc_canister_id: p(21),
            cmc_to_account_owner: p(21),
            cmc_to_account_subaccount: Some([22; 32]),
            amount_e8s: id.saturating_mul(100),
            fee_e8s: 10_000,
            memo: vec![23],
            created_at_time_ns: id,
            ledger_block_index: None,
            notify_attempts: 0,
            cycles_sent: None,
            status: IcpRefillRecordStatus::Requested,
            error_code: None,
            error_message: None,
            refund_block_index: None,
            transaction_too_old_min_block_index: None,
            created_at_ns: id,
            updated_at_ns: id,
        }
    }

    fn reset_records() {
        IcpRefillRecords::clear_for_tests();
        ICP_REFILL_DERIVED_INDEX.with_borrow_mut(|index| *index = IcpRefillDerivedIndex::default());
    }

    #[test]
    fn rebuild_restores_bounded_lookup_id_and_metric_indexes() {
        let _guard = seams::lock();
        reset_records();
        let target = p(30);
        let mut completed = record(1, [1; 32], target);
        completed.status = IcpRefillRecordStatus::Completed;
        completed.cycles_sent = Some(Nat::from(4_000_u64));
        let requested = record(2, [2; 32], target);
        let _ = IcpRefillRecords::insert(completed);
        let _ = IcpRefillRecords::insert(requested.clone());

        IcpRefillRecordOps::rebuild_indexes().expect("rebuild derived indexes");

        assert_eq!(
            IcpRefillRecordOps::find_by_operation_id([1; 32])
                .expect("operation lookup")
                .map(|record| record.id),
            Some(1)
        );
        assert!(
            IcpRefillRecordOps::has_active_for_key(
                requested.source_canister,
                requested.source_subaccount,
                requested.target_canister,
                [9; 32],
            )
            .expect("active lookup")
        );
        assert_eq!(IcpRefillRecordOps::resumable_operation_count(), 1);
        assert_eq!(next_id().expect("next id"), 3);
        let metrics = IcpRefillRecordOps::metric_snapshot();
        assert_eq!(metrics.targets.len(), 1);
        assert_eq!(metrics.targets[0].amount_e8s, 300);
        assert_eq!(metrics.targets[0].cycles_sent, Some(4_000));
        reset_records();
    }

    #[test]
    fn rebuild_rejects_duplicate_operation_identity() {
        let _guard = seams::lock();
        reset_records();
        let mut first = record(1, [7; 32], p(31));
        first.status = IcpRefillRecordStatus::Completed;
        let mut second = record(2, [7; 32], p(32));
        second.status = IcpRefillRecordStatus::Completed;
        let _ = IcpRefillRecords::insert(first);
        let _ = IcpRefillRecords::insert(second);

        let err = IcpRefillRecordOps::rebuild_indexes()
            .expect_err("duplicate operation identity must fail closed");

        assert!(matches!(
            err,
            IcpRefillRecordOpsError::DuplicateOperationIndex {
                existing_id: 1,
                conflicting_id: 2,
            }
        ));
        reset_records();
    }

    #[test]
    fn rebuild_rejects_completed_cycles_sent_overflow() {
        let _guard = seams::lock();
        reset_records();
        let mut completed = record(1, [6; 32], p(30));
        completed.status = IcpRefillRecordStatus::Completed;
        completed.cycles_sent = Some(
            Nat::parse(b"340282366920938463463374607431768211456")
                .expect("u128 max plus one is valid Nat"),
        );
        let _ = IcpRefillRecords::insert(completed);

        let err = IcpRefillRecordOps::rebuild_indexes()
            .expect_err("oversized completed cycles must fail closed");

        assert!(matches!(
            err,
            IcpRefillRecordOpsError::CyclesSentOverflow { id: 1, .. }
        ));
        reset_records();
    }

    #[test]
    fn rebuild_rejects_duplicate_active_refill_identity() {
        let _guard = seams::lock();
        reset_records();
        let first = record(1, [7; 32], p(31));
        let mut second = record(2, [8; 32], first.target_canister);
        second.source_canister = first.source_canister;
        second.source_subaccount = first.source_subaccount;
        let _ = IcpRefillRecords::insert(first);
        let _ = IcpRefillRecords::insert(second);

        let err = IcpRefillRecordOps::rebuild_indexes()
            .expect_err("duplicate active refill identity must fail closed");

        assert!(matches!(
            err,
            IcpRefillRecordOpsError::DuplicateActiveIndex {
                existing_id: 1,
                conflicting_id: 2,
            }
        ));
        reset_records();
    }

    #[test]
    fn terminal_transition_removes_active_index_and_updates_metrics() {
        let _guard = seams::lock();
        reset_records();
        let requested = record(1, [8; 32], p(33));
        IcpRefillRecordOps::insert(requested.clone()).expect("insert requested record");
        assert!(
            IcpRefillRecordOps::has_active_for_key(
                requested.source_canister,
                requested.source_subaccount,
                requested.target_canister,
                [9; 32],
            )
            .expect("active lookup")
        );

        IcpRefillRecordOps::mark_completed(1, 5_000, 2).expect("complete refill");

        assert!(
            !IcpRefillRecordOps::has_active_for_key(
                requested.source_canister,
                requested.source_subaccount,
                requested.target_canister,
                [9; 32],
            )
            .expect("active lookup")
        );
        assert_eq!(IcpRefillRecordOps::resumable_operation_count(), 0);
        let metrics = IcpRefillRecordOps::metric_snapshot();
        assert_eq!(metrics.targets[0].cycles_sent, Some(5_000));
        assert!(metrics.statuses.iter().any(|status| {
            status.status == IcpRefillRecordStatus::Completed && status.count == 1
        }));
        reset_records();
    }
}
