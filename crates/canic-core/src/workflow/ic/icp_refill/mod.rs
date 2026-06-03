use crate::{
    InternalError, InternalErrorOrigin,
    cdk::{
        candid::Nat,
        icrc_ledger_types::icrc1::transfer::TransferError,
        types::{Cycles, Principal},
    },
    config::schema::{IcpRefillPolicy, TopupPolicy},
    domain::policy::{
        cycles_funding,
        icp_refill::{
            IcpRefillPolicyInput, IcpRefillPolicyViolation, evaluate_hub_self_refill,
            evaluate_manual_refill,
        },
    },
    dto::icp_refill::{
        IcpRefillDryRun, IcpRefillErrorCode, IcpRefillMode, IcpRefillRequest, IcpRefillResponse,
        IcpRefillStatus,
    },
    ids::BuildNetwork,
    infra::ic::icp_refill::{IcpRefillCanisterOverrides, NotifyTopUpArg, NotifyTopUpError},
    ops::{
        config::ConfigOps,
        ic::{IcOps, icp_refill::IcpRefillOps},
        runtime::cycles_funding::CyclesFundingLedgerOps,
        storage::{
            children::CanisterChildrenOps,
            icp_refill::{IcpRefillRecordCreateInput, IcpRefillRecordOps},
            state::app::AppStateOps,
        },
    },
    storage::stable::icp_refill::IcpRefillRecord,
    workflow::ic::network::NetworkWorkflow,
};
use sha2::{Digest, Sha256};
use thiserror::Error as ThisError;

const TX_WINDOW_NANOS: u64 = 24 * 60 * 60 * 1_000_000_000;
const MAX_NOTIFY_ATTEMPTS: u32 = 5;
const ICP_LEDGER_DECIMALS: u8 = 8;

///
/// IcpRefillWorkflowError
///

#[derive(Debug, ThisError)]
pub enum IcpRefillWorkflowError {
    #[error("ICP refill only supports canister mode in this workflow")]
    UnsupportedMode,

    #[error("ICP refill source canister {source_canister} must be this canister {self_pid}")]
    SourceCanisterMismatch {
        source_canister: Principal,
        self_pid: Principal,
    },

    #[error("ICP refill request is marked dry_run; call dry_run_manual_refill instead")]
    DryRunRequest,

    #[error("ICP refill policy denied request: {0:?}")]
    PolicyDenied(IcpRefillPolicyViolation),

    #[error(
        "ICP refill retry request does not match stored operation {field}: request={request_value}, record={record_value}"
    )]
    RetryRequestMismatch {
        field: &'static str,
        request_value: String,
        record_value: String,
    },

    #[error("ICP refill Nat field {field} does not fit in u64: {value}")]
    NatU64Overflow { field: &'static str, value: Nat },

    #[error("ICP refill expected ICP ledger decimals=8, found {0}")]
    UnexpectedLedgerDecimals(u8),
}

impl From<IcpRefillWorkflowError> for InternalError {
    fn from(err: IcpRefillWorkflowError) -> Self {
        Self::workflow(InternalErrorOrigin::Workflow, err.to_string())
    }
}

///
/// IcpRefillWorkflow
///

pub struct IcpRefillWorkflow;

impl IcpRefillWorkflow {
    pub async fn dry_run_manual_refill(
        request: IcpRefillRequest,
    ) -> Result<IcpRefillDryRun, InternalError> {
        validate_manual_request_shape(&request, true)?;
        let context = prepare_context(&request, RateQueryMode::Always).await?;

        Ok(IcpRefillDryRun {
            operation_id: request.operation_id,
            mode: request.mode,
            amount_e8s: request.amount_e8s,
            fee_e8s: context.fee_e8s,
            xdr_permyriad_per_icp: context.xdr_permyriad_per_icp,
            estimated_cycles: context
                .xdr_permyriad_per_icp
                .map(|rate| estimate_cycles(request.amount_e8s, rate)),
            message: dry_run_message(request.mode),
        })
    }

    pub async fn execute_manual_refill(
        request: IcpRefillRequest,
    ) -> Result<IcpRefillResponse, InternalError> {
        validate_manual_request_shape(&request, false)?;
        if let Some(record) = IcpRefillRecordOps::find_by_operation_id(request.operation_id) {
            validate_retry_request_matches_record(&request, &record)?;
            let record = advance_record(record).await?;
            return Ok(IcpRefillRecordOps::to_response(&record));
        }

        let context = prepare_context(&request, RateQueryMode::WhenGateConfigured).await?;
        let cmc_account =
            IcpRefillOps::cmc_topup_account(context.cmc_canister_id, request.target_canister)?;
        let record = IcpRefillRecordOps::create_or_get(IcpRefillRecordCreateInput {
            operation_id: request.operation_id,
            source_canister: request.source_canister,
            source_subaccount: request.source_subaccount,
            target_canister: request.target_canister,
            ledger_canister_id: context.ledger_canister_id,
            cmc_canister_id: context.cmc_canister_id,
            cmc_to_account_owner: cmc_account.owner,
            cmc_to_account_subaccount: cmc_account.subaccount,
            amount_e8s: request.amount_e8s,
            fee_e8s: context.fee_e8s,
            memo: IcpRefillOps::topup_memo(),
            created_at_time_ns: context.created_at_time_ns,
            now_ns: IcOps::now_nanos(),
        })?;

        let record = advance_record(record).await?;

        Ok(IcpRefillRecordOps::to_response(&record))
    }

    pub async fn execute_hub_self_refill(
        hub_cycles: Cycles,
    ) -> Result<IcpRefillResponse, InternalError> {
        let self_pid = IcOps::canister_self();
        if let Some(record) = find_resumable_hub_self_refill(self_pid) {
            let request = request_from_record(&record);
            return Self::execute_manual_refill(request).await;
        }

        let Some(topup) = current_topup_policy()? else {
            return Err(policy_denied(IcpRefillPolicyViolation::NotConfigured));
        };
        let Some(icp_refill) = topup.icp_refill.as_ref() else {
            return Err(policy_denied(IcpRefillPolicyViolation::NotConfigured));
        };
        let canisters = IcpRefillOps::resolve_canisters(
            build_network(),
            IcpRefillCanisterOverrides::default(),
        )?;
        let observed_rate = configured_rate(
            Some(icp_refill),
            canisters.cmc_canister_id,
            RateQueryMode::WhenGateConfigured,
        )
        .await?;
        let now_ns = IcOps::now_nanos();
        let now_secs = IcOps::now_secs();
        let request = IcpRefillRequest {
            operation_id: hub_self_refill_operation_id(
                self_pid,
                None,
                self_pid,
                icp_refill.max_refill_e8s_per_call,
                now_ns,
            ),
            source_canister: self_pid,
            source_subaccount: None,
            target_canister: self_pid,
            amount_e8s: icp_refill.max_refill_e8s_per_call,
            dry_run: false,
            mode: IcpRefillMode::Canister,
        };

        evaluate_hub_self_refill(
            Some(&topup),
            policy_input(
                hub_cycles.to_u128(),
                &request,
                observed_rate,
                has_in_flight_record(&request),
                AppStateOps::cycles_funding_enabled(),
                funding_cooldown_retry_after_secs(&request, now_secs),
            ),
        )
        .map_err(policy_denied)?;

        Self::execute_manual_refill(request).await
    }
}

///
/// IcpRefillExecutionContext
///

struct IcpRefillExecutionContext {
    ledger_canister_id: Principal,
    cmc_canister_id: Principal,
    fee_e8s: u64,
    xdr_permyriad_per_icp: Option<u64>,
    created_at_time_ns: u64,
}

///
/// RateQueryMode
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RateQueryMode {
    Always,
    WhenGateConfigured,
}

async fn prepare_context(
    request: &IcpRefillRequest,
    rate_query_mode: RateQueryMode,
) -> Result<IcpRefillExecutionContext, InternalError> {
    let policy = current_icp_refill_policy()?;
    let in_flight_for_key = has_in_flight_record(request);
    let cycles_funding_enabled = AppStateOps::cycles_funding_enabled();
    let funding_cooldown_retry_after_secs =
        funding_cooldown_retry_after_secs(request, IcOps::now_secs());
    let rate_gate_configured = policy_requires_rate(policy.as_ref());
    if !rate_gate_configured {
        evaluate_manual_refill(
            policy.as_ref(),
            policy_input(
                0,
                request,
                None,
                in_flight_for_key,
                cycles_funding_enabled,
                funding_cooldown_retry_after_secs,
            ),
        )
        .map_err(policy_denied)?;
    }

    let canisters =
        IcpRefillOps::resolve_canisters(build_network(), IcpRefillCanisterOverrides::default())?;
    let fee = IcpRefillOps::icrc1_fee(canisters.ledger_canister_id).await?;
    let fee_e8s = checked_nat_u64("icrc1_fee", fee)?;
    validate_ledger_decimals(IcpRefillOps::icrc1_decimals(canisters.ledger_canister_id).await?)?;
    let xdr_permyriad_per_icp =
        configured_rate(policy.as_ref(), canisters.cmc_canister_id, rate_query_mode).await?;

    if rate_gate_configured {
        evaluate_manual_refill(
            policy.as_ref(),
            policy_input(
                0,
                request,
                xdr_permyriad_per_icp,
                in_flight_for_key,
                cycles_funding_enabled,
                funding_cooldown_retry_after_secs,
            ),
        )
        .map_err(policy_denied)?;
    }

    Ok(IcpRefillExecutionContext {
        ledger_canister_id: canisters.ledger_canister_id,
        cmc_canister_id: canisters.cmc_canister_id,
        fee_e8s,
        xdr_permyriad_per_icp,
        created_at_time_ns: IcOps::now_nanos(),
    })
}

async fn transfer_record(record: IcpRefillRecord) -> Result<IcpRefillRecord, InternalError> {
    let to = IcpRefillOps::cmc_topup_account(record.cmc_canister_id, record.target_canister)?;
    let transfer_arg = IcpRefillOps::transfer_arg(
        record.source_subaccount,
        to,
        record.amount_e8s,
        record.fee_e8s,
        record.memo.clone(),
        record.created_at_time_ns,
    );

    match IcpRefillOps::icrc1_transfer(record.ledger_canister_id, transfer_arg).await? {
        Ok(block_index) => {
            let block_index = match IcpRefillOps::checked_block_index(block_index) {
                Ok(block_index) => block_index,
                Err(err) => {
                    return IcpRefillRecordOps::mark_transfer_failed(
                        record.id,
                        IcpRefillErrorCode::InvalidLedgerBlockIndex,
                        err.to_string(),
                        IcOps::now_nanos(),
                    );
                }
            };
            IcpRefillRecordOps::mark_transferred(record.id, block_index, IcOps::now_nanos())
        }
        Err(err) => apply_transfer_error(record.id, err),
    }
}

async fn advance_record(record: IcpRefillRecord) -> Result<IcpRefillRecord, InternalError> {
    let record = match record.status {
        IcpRefillStatus::Requested => transfer_unless_window_stale(record).await?,
        IcpRefillStatus::Transferred | IcpRefillStatus::NotifyProcessing => record,
        IcpRefillStatus::Failed if can_retry_notify(&record) => record,
        IcpRefillStatus::Failed if can_retry_bad_fee(&record) => {
            transfer_unless_window_stale(record).await?
        }
        IcpRefillStatus::Completed
        | IcpRefillStatus::Failed
        | IcpRefillStatus::InvalidTransaction
        | IcpRefillStatus::Refunded
        | IcpRefillStatus::TransactionTooOld => return Ok(record),
    };

    if should_notify(&record) {
        notify_record(record).await
    } else {
        Ok(record)
    }
}

async fn transfer_unless_window_stale(
    record: IcpRefillRecord,
) -> Result<IcpRefillRecord, InternalError> {
    let now_ns = IcOps::now_nanos();
    if transfer_window_stale(&record, now_ns) {
        IcpRefillRecordOps::mark_transfer_window_stale(record.id, now_ns)
    } else {
        transfer_record(record).await
    }
}

async fn notify_record(record: IcpRefillRecord) -> Result<IcpRefillRecord, InternalError> {
    let Some(block_index) = record.ledger_block_index else {
        return IcpRefillRecordOps::mark_notify_failed(
            record.id,
            "notify_top_up cannot run before ledger block is recorded".to_string(),
            IcOps::now_nanos(),
        );
    };

    let record = IcpRefillRecordOps::mark_notify_attempt_started(record.id, IcOps::now_nanos())?;
    let args = NotifyTopUpArg {
        block_index,
        canister_id: record.target_canister,
    };

    match IcpRefillOps::notify_top_up(record.cmc_canister_id, args).await {
        Ok(Ok(cycles_sent)) => {
            let record =
                IcpRefillRecordOps::mark_completed(record.id, cycles_sent, IcOps::now_nanos())?;
            record_direct_child_refill_grant(&record, IcOps::now_secs());
            Ok(record)
        }
        Ok(Err(err)) => apply_notify_error(record.id, record.notify_attempts, err),
        Err(err) => {
            mark_retryable_notify_failure(record.id, record.notify_attempts, err.to_string())
        }
    }
}

fn apply_transfer_error(
    record_id: u64,
    err: TransferError,
) -> Result<IcpRefillRecord, InternalError> {
    match err {
        TransferError::BadFee { expected_fee } => {
            let expected_fee_e8s = match checked_nat_u64("bad_fee.expected_fee", expected_fee) {
                Ok(expected_fee_e8s) => expected_fee_e8s,
                Err(err) => {
                    return IcpRefillRecordOps::mark_transfer_failed(
                        record_id,
                        IcpRefillErrorCode::BadFee,
                        err.to_string(),
                        IcOps::now_nanos(),
                    );
                }
            };
            IcpRefillRecordOps::mark_bad_fee(
                record_id,
                expected_fee_e8s,
                format!("bad fee; expected {expected_fee_e8s}"),
                IcOps::now_nanos(),
            )
        }
        TransferError::Duplicate { duplicate_of } => {
            let duplicate_of = match IcpRefillOps::checked_block_index(duplicate_of) {
                Ok(block_index) => block_index,
                Err(err) => {
                    return IcpRefillRecordOps::mark_transfer_failed(
                        record_id,
                        IcpRefillErrorCode::InvalidLedgerBlockIndex,
                        err.to_string(),
                        IcOps::now_nanos(),
                    );
                }
            };
            IcpRefillRecordOps::mark_duplicate_transferred(
                record_id,
                duplicate_of,
                IcOps::now_nanos(),
            )
        }
        TransferError::TooOld => {
            IcpRefillRecordOps::mark_transfer_window_stale(record_id, IcOps::now_nanos())
        }
        other => IcpRefillRecordOps::mark_transfer_failed(
            record_id,
            IcpRefillErrorCode::LedgerTransferFailed,
            other.to_string(),
            IcOps::now_nanos(),
        ),
    }
}

fn apply_notify_error(
    record_id: u64,
    notify_attempts: u32,
    err: NotifyTopUpError,
) -> Result<IcpRefillRecord, InternalError> {
    match err {
        NotifyTopUpError::Refunded {
            block_index,
            reason,
        } => IcpRefillRecordOps::mark_refunded(record_id, block_index, reason, IcOps::now_nanos()),
        NotifyTopUpError::InvalidTransaction(reason) => {
            IcpRefillRecordOps::mark_invalid_transaction(record_id, reason, IcOps::now_nanos())
        }
        NotifyTopUpError::Processing => mark_notify_processing(record_id, notify_attempts),
        NotifyTopUpError::TransactionTooOld(min_block_index) => {
            IcpRefillRecordOps::mark_transaction_too_old(
                record_id,
                Some(min_block_index),
                IcOps::now_nanos(),
            )
        }
        NotifyTopUpError::Other {
            error_code,
            error_message,
        } => mark_retryable_notify_failure(
            record_id,
            notify_attempts,
            format!("notify_top_up error {error_code}: {error_message}"),
        ),
    }
}

fn mark_notify_processing(
    record_id: u64,
    notify_attempts: u32,
) -> Result<IcpRefillRecord, InternalError> {
    if notify_attempts >= MAX_NOTIFY_ATTEMPTS {
        IcpRefillRecordOps::mark_notify_max_attempts(
            record_id,
            "notify_top_up returned Processing after max attempts".to_string(),
            IcOps::now_nanos(),
        )
    } else {
        IcpRefillRecordOps::mark_notify_processing(record_id, IcOps::now_nanos())
    }
}

fn mark_retryable_notify_failure(
    record_id: u64,
    notify_attempts: u32,
    error_message: String,
) -> Result<IcpRefillRecord, InternalError> {
    if notify_attempts >= MAX_NOTIFY_ATTEMPTS {
        IcpRefillRecordOps::mark_notify_max_attempts(record_id, error_message, IcOps::now_nanos())
    } else {
        IcpRefillRecordOps::mark_notify_failed(record_id, error_message, IcOps::now_nanos())
    }
}

fn validate_manual_request_shape(
    request: &IcpRefillRequest,
    allow_dry_run: bool,
) -> Result<(), InternalError> {
    if request.mode != IcpRefillMode::Canister {
        return Err(IcpRefillWorkflowError::UnsupportedMode.into());
    }
    if request.dry_run && !allow_dry_run {
        return Err(IcpRefillWorkflowError::DryRunRequest.into());
    }
    let self_pid = IcOps::canister_self();
    if request.source_canister != self_pid {
        return Err(IcpRefillWorkflowError::SourceCanisterMismatch {
            source_canister: request.source_canister,
            self_pid,
        }
        .into());
    }

    Ok(())
}

async fn configured_rate(
    policy: Option<&IcpRefillPolicy>,
    cmc_canister_id: Principal,
    mode: RateQueryMode,
) -> Result<Option<u64>, InternalError> {
    if !rate_required(policy, mode) {
        return Ok(None);
    }

    let response = IcpRefillOps::get_icp_xdr_conversion_rate(cmc_canister_id).await?;
    Ok(Some(response.data.xdr_permyriad_per_icp))
}

const fn policy_requires_rate(policy: Option<&IcpRefillPolicy>) -> bool {
    matches!(
        policy,
        Some(IcpRefillPolicy {
            min_xdr_permyriad_per_icp: Some(_),
            ..
        })
    )
}

const fn rate_required(policy: Option<&IcpRefillPolicy>, mode: RateQueryMode) -> bool {
    matches!(mode, RateQueryMode::Always) || policy_requires_rate(policy)
}

fn validate_ledger_decimals(decimals: u8) -> Result<(), InternalError> {
    if decimals == ICP_LEDGER_DECIMALS {
        Ok(())
    } else {
        Err(IcpRefillWorkflowError::UnexpectedLedgerDecimals(decimals).into())
    }
}

fn estimate_cycles(amount_e8s: u64, xdr_permyriad_per_icp: u64) -> Cycles {
    Cycles::new(u128::from(amount_e8s).saturating_mul(u128::from(xdr_permyriad_per_icp)))
}

fn current_icp_refill_policy() -> Result<Option<IcpRefillPolicy>, InternalError> {
    Ok(ConfigOps::current_canister()?
        .topup
        .and_then(|topup| topup.icp_refill))
}

fn current_topup_policy() -> Result<Option<TopupPolicy>, InternalError> {
    Ok(ConfigOps::current_canister()?.topup)
}

const fn policy_input(
    hub_cycles: u128,
    request: &IcpRefillRequest,
    observed_xdr_permyriad_per_icp: Option<u64>,
    in_flight_for_key: bool,
    cycles_funding_enabled: bool,
    funding_cooldown_retry_after_secs: Option<u64>,
) -> IcpRefillPolicyInput {
    IcpRefillPolicyInput {
        hub_cycles,
        requested_amount_e8s: request.amount_e8s,
        observed_xdr_permyriad_per_icp,
        in_flight_for_key,
        cycles_funding_enabled,
        funding_cooldown_retry_after_secs,
    }
}

fn funding_cooldown_retry_after_secs(request: &IcpRefillRequest, now_secs: u64) -> Option<u64> {
    let (role, parent_pid) = CanisterChildrenOps::role_parent(request.target_canister)?;
    if parent_pid != Some(request.source_canister) {
        return None;
    }

    cycles_funding::policy_for_child_role(&role).cooldown_retry_after_secs(
        CyclesFundingLedgerOps::snapshot(request.target_canister),
        now_secs,
    )
}

fn record_direct_child_refill_grant(record: &IcpRefillRecord, now_secs: u64) {
    let Some(cycles_sent) = record.cycles_sent.as_ref() else {
        return;
    };
    let Some((_role, parent_pid)) = CanisterChildrenOps::role_parent(record.target_canister) else {
        return;
    };
    let Some((child, cycles)) = direct_child_refill_grant(record, cycles_sent, parent_pid) else {
        return;
    };

    CyclesFundingLedgerOps::record_child_grant(child, cycles, now_secs);
}

fn direct_child_refill_grant(
    record: &IcpRefillRecord,
    cycles_sent: &Nat,
    parent_pid: Option<Principal>,
) -> Option<(Principal, u128)> {
    if parent_pid != Some(record.source_canister) {
        return None;
    }

    Some((
        record.target_canister,
        u128::try_from(cycles_sent.0.clone()).unwrap_or(u128::MAX),
    ))
}

fn policy_denied(violation: IcpRefillPolicyViolation) -> InternalError {
    IcpRefillWorkflowError::PolicyDenied(violation).into()
}

fn has_in_flight_record(request: &IcpRefillRequest) -> bool {
    IcpRefillRecordOps::records().into_iter().any(|record| {
        record.source_canister == request.source_canister
            && record.source_subaccount == request.source_subaccount
            && record.target_canister == request.target_canister
            && is_in_flight(record.status)
            && record.operation_id != request.operation_id
    })
}

fn find_resumable_hub_self_refill(self_pid: Principal) -> Option<IcpRefillRecord> {
    IcpRefillRecordOps::records().into_iter().find(|record| {
        record.source_canister == self_pid
            && record.source_subaccount.is_none()
            && record.target_canister == self_pid
            && is_resumable(record)
    })
}

const fn request_from_record(record: &IcpRefillRecord) -> IcpRefillRequest {
    IcpRefillRequest {
        operation_id: record.operation_id,
        source_canister: record.source_canister,
        source_subaccount: record.source_subaccount,
        target_canister: record.target_canister,
        amount_e8s: record.amount_e8s,
        dry_run: false,
        mode: IcpRefillMode::Canister,
    }
}

fn validate_retry_request_matches_record(
    request: &IcpRefillRequest,
    record: &IcpRefillRecord,
) -> Result<(), InternalError> {
    ensure_retry_field(
        "source_canister",
        request.source_canister,
        record.source_canister,
    )?;
    ensure_retry_field(
        "source_subaccount",
        request.source_subaccount,
        record.source_subaccount,
    )?;
    ensure_retry_field(
        "target_canister",
        request.target_canister,
        record.target_canister,
    )?;
    ensure_retry_field("amount_e8s", request.amount_e8s, record.amount_e8s)?;

    Ok(())
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

    Err(IcpRefillWorkflowError::RetryRequestMismatch {
        field,
        request_value: format!("{request_value:?}"),
        record_value: format!("{record_value:?}"),
    }
    .into())
}

const fn is_in_flight(status: IcpRefillStatus) -> bool {
    matches!(
        status,
        IcpRefillStatus::Requested
            | IcpRefillStatus::Transferred
            | IcpRefillStatus::NotifyProcessing
    )
}

const fn is_resumable(record: &IcpRefillRecord) -> bool {
    is_in_flight(record.status) || can_retry_notify(record) || can_retry_bad_fee(record)
}

const fn can_retry_notify(record: &IcpRefillRecord) -> bool {
    record.ledger_block_index.is_some()
        && matches!(record.status, IcpRefillStatus::Failed)
        && matches!(record.error_code, Some(IcpRefillErrorCode::NotifyFailed))
}

const fn can_retry_bad_fee(record: &IcpRefillRecord) -> bool {
    record.ledger_block_index.is_none()
        && matches!(record.status, IcpRefillStatus::Failed)
        && matches!(record.error_code, Some(IcpRefillErrorCode::BadFee))
}

const fn should_notify(record: &IcpRefillRecord) -> bool {
    record.ledger_block_index.is_some()
        && (matches!(
            record.status,
            IcpRefillStatus::Transferred | IcpRefillStatus::NotifyProcessing
        ) || can_retry_notify(record))
}

const fn transfer_window_stale(record: &IcpRefillRecord, now_ns: u64) -> bool {
    record.ledger_block_index.is_none()
        && (matches!(record.status, IcpRefillStatus::Requested) || can_retry_bad_fee(record))
        && record.created_at_time_ns.saturating_add(TX_WINDOW_NANOS) < now_ns
}

fn build_network() -> BuildNetwork {
    NetworkWorkflow::build_network().unwrap_or(BuildNetwork::Local)
}

fn checked_nat_u64(field: &'static str, value: Nat) -> Result<u64, InternalError> {
    u64::try_from(value.0.clone())
        .map_err(|_| IcpRefillWorkflowError::NatU64Overflow { field, value }.into())
}

fn dry_run_message(mode: IcpRefillMode) -> Option<String> {
    match mode {
        IcpRefillMode::Canister => None,
        IcpRefillMode::Fabricate => {
            Some("mode=fabricate (does not call canister refill endpoint)".to_string())
        }
    }
}

fn hub_self_refill_operation_id(
    source_canister: Principal,
    source_subaccount: Option<[u8; 32]>,
    target_canister: Principal,
    amount_e8s: u64,
    now_ns: u64,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"canic:icp-refill:hub-self-refill:v1");
    hasher.update(source_canister.as_slice());
    hasher.update(source_subaccount.unwrap_or_default());
    hasher.update(target_canister.as_slice());
    hasher.update(amount_e8s.to_be_bytes());
    hasher.update(now_ns.to_be_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests;
