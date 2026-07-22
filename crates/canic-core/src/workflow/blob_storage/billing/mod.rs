//! Module: workflow::blob_storage::billing
//!
//! Responsibility: orchestrate configuration, Cashier calls, funding, and readiness.
//! Does not own: endpoint authorization, pure billing decisions, or stable records.
//! Boundary: API delegates here; workflow sequences policy and single-step ops.

use crate::{
    InternalError,
    cdk::types::Principal,
    domain::{
        blob_storage::{BlobStorageFundingStatus, BlobStoragePaymentModelStatus},
        policy::pure::blob_storage::{
            BlobStorageBillingPolicyError, BlobStorageCashierBalanceObservation,
            BlobStorageFundingStatusDecision, BlobStorageStatusDecision,
            decide_blob_storage_funding_attachment, decide_configured_blob_storage_status,
            decide_unconfigured_blob_storage_status, validate_blob_storage_billing_balances,
            validate_blob_storage_billing_config_header, validate_blob_storage_funding_request,
        },
    },
    dto::blob_storage::{
        BlobProjectCyclesTopUpReport, BlobStorageBillingConfig,
        BlobStorageCashierAccountBalanceGetError, BlobStorageCashierAccountBalanceGetResult,
        BlobStorageCashierAccountTopUpError, BlobStorageCashierAccountTopUpRequest,
        BlobStorageCashierAccountTopUpResult, BlobStorageStatusRequest, BlobStorageStatusResponse,
    },
    ops::{
        blob_storage::{
            conversion::{BlobStorageConversionError, BlobStorageConversionOps},
            funding::{BlobStorageFundingInProgress, BlobStorageFundingOps},
            lifecycle::BlobStorageLifecycleOps,
        },
        cashier::{
            client::CashierClientOps,
            conversion::{CashierConversionOps, CashierDecodeError},
        },
        ic::IcOps,
    },
    view::blob_storage::BlobStorageBillingConfigView,
};
use thiserror::Error as ThisError;

/// Canonical blob-storage billing workflow.
pub struct BlobStorageBillingWorkflow;

impl BlobStorageBillingWorkflow {
    /// Validate and persist one billing configuration.
    pub fn configure_billing(
        config: BlobStorageBillingConfig,
    ) -> Result<(), BlobStorageBillingWorkflowError> {
        validate_blob_storage_billing_config_header(
            config.cashier_canister_id,
            config.gateway_principal_limit,
        )?;
        let _gateway_principal_limit = BlobStorageConversionOps::gateway_principal_limit_to_usize(
            config.gateway_principal_limit,
        )?;
        let project_cycles_reserve = BlobStorageConversionOps::billing_nat_to_u128(
            "project_cycles_reserve",
            &config.project_cycles_reserve,
        )?;
        let min_upload_balance = BlobStorageConversionOps::billing_nat_to_u128(
            "min_upload_balance",
            &config.min_upload_balance,
        )?;
        let target_upload_balance = BlobStorageConversionOps::billing_nat_to_u128(
            "target_upload_balance",
            &config.target_upload_balance,
        )?;
        validate_blob_storage_billing_balances(
            project_cycles_reserve,
            min_upload_balance,
            target_upload_balance,
        )?;

        BlobStorageLifecycleOps::set_billing_config(
            config.cashier_canister_id,
            project_cycles_reserve,
            min_upload_balance,
            target_upload_balance,
            config.gateway_principal_limit,
            IcOps::now_nanos(),
        );
        Ok(())
    }

    /// Return the configured billing contract, if present.
    #[must_use]
    pub fn billing_config() -> Option<BlobStorageBillingConfig> {
        BlobStorageLifecycleOps::billing_config_dto()
    }

    /// Perform the raw Cashier balance call.
    pub async fn cashier_account_balance_get(
        cashier_canister_id: Principal,
        account: Principal,
    ) -> Result<BlobStorageCashierAccountBalanceGetResult, BlobStorageBillingWorkflowError> {
        CashierClientOps::account_balance_get(cashier_canister_id, account)
            .await
            .map_err(Into::into)
    }

    /// Read and decode the total Cashier balance for one account.
    pub async fn cashier_account_total_balance(
        cashier_canister_id: Principal,
        account: Principal,
    ) -> Result<u128, BlobStorageBillingWorkflowError> {
        match Self::cashier_account_balance_get(cashier_canister_id, account).await? {
            BlobStorageCashierAccountBalanceGetResult::Ok(balance) => {
                CashierConversionOps::account_cycle_balances_to_u128(
                    &balance.account_cycle_balances,
                )
                .map(|balances| balances.total)
                .map_err(Into::into)
            }
            BlobStorageCashierAccountBalanceGetResult::Err(
                BlobStorageCashierAccountBalanceGetError::AccountNotFound,
            ) => Ok(0),
            BlobStorageCashierAccountBalanceGetResult::Err(
                BlobStorageCashierAccountBalanceGetError::InternalError(message),
            ) => Err(BlobStorageBillingWorkflowError::CashierBalanceInternal(
                message,
            )),
        }
    }

    /// Perform the raw Cashier top-up call with an admitted cycle amount.
    pub async fn cashier_account_top_up(
        cashier_canister_id: Principal,
        request: Option<BlobStorageCashierAccountTopUpRequest>,
        cycles: u128,
    ) -> Result<BlobStorageCashierAccountTopUpResult, BlobStorageBillingWorkflowError> {
        CashierClientOps::account_top_up(cashier_canister_id, request, cycles)
            .await
            .map_err(Into::into)
    }

    /// Replace local gateway authority from one bounded Cashier observation.
    pub async fn sync_gateway_principals_from_cashier(
        cashier_canister_id: Principal,
        max_gateway_principals: usize,
    ) -> Result<u64, BlobStorageBillingWorkflowError> {
        let principals =
            CashierClientOps::storage_gateway_principal_list(cashier_canister_id).await?;
        let principals =
            CashierConversionOps::normalize_gateway_principals(principals, max_gateway_principals)?;

        let now_ns = IcOps::now_nanos();
        let count = BlobStorageLifecycleOps::replace_gateway_principals(&principals, now_ns);
        BlobStorageLifecycleOps::record_gateway_principal_sync(now_ns);
        Ok(count)
    }

    /// Replace local gateway authority from the configured Cashier.
    pub async fn sync_gateway_principals_from_configured_cashier()
    -> Result<u64, BlobStorageBillingWorkflowError> {
        let config = Self::require_billing_config()?;
        let max_gateway_principals = BlobStorageConversionOps::gateway_principal_limit_to_usize(
            config.gateway_principal_limit,
        )?;

        Self::sync_gateway_principals_from_cashier(
            config.cashier_canister_id,
            max_gateway_principals,
        )
        .await
    }

    /// Fund the configured Cashier account while retaining the project reserve.
    pub async fn fund_from_project_cycles(
        requested_cycles: u128,
    ) -> Result<BlobProjectCyclesTopUpReport, BlobStorageBillingWorkflowError> {
        let config = Self::require_billing_config()?;
        validate_blob_storage_funding_request(requested_cycles)?;
        let _funding_guard = BlobStorageFundingOps::try_acquire()?;

        let attachment = decide_blob_storage_funding_attachment(
            requested_cycles,
            IcOps::canister_cycle_balance().to_u128(),
            config.project_cycles_reserve,
        );
        let project_cycles_before = attachment.project_cycles_available;
        let attached_cycles = attachment.attached_cycles;

        if attached_cycles == 0 {
            return Ok(Self::top_up_report(
                requested_cycles,
                0,
                project_cycles_before,
                project_cycles_before,
                config.project_cycles_reserve,
                0,
                attachment.skipped_reason.map(str::to_string),
            ));
        }

        let account = IcOps::canister_self();
        let result = Self::cashier_account_top_up(
            config.cashier_canister_id,
            Some(BlobStorageCashierAccountTopUpRequest {
                target_balance: None,
                account: Some(account),
            }),
            attached_cycles,
        )
        .await?;
        let top_up = match result {
            BlobStorageCashierAccountTopUpResult::Ok(top_up) => top_up,
            BlobStorageCashierAccountTopUpResult::Err(err) => {
                return Err(BlobStorageBillingWorkflowError::CashierTopUp(err));
            }
        };
        let cashier_total_after =
            CashierConversionOps::account_cycle_balances_to_u128(&top_up.balance)?.total;
        let project_cycles_after = IcOps::canister_cycle_balance().to_u128();

        Ok(Self::top_up_report(
            requested_cycles,
            attached_cycles,
            project_cycles_before,
            project_cycles_after,
            config.project_cycles_reserve,
            cashier_total_after,
            None,
        ))
    }

    /// Observe billing readiness without mutating gateway or funding state.
    pub async fn status(request: BlobStorageStatusRequest) -> BlobStorageStatusResponse {
        let project_cycles_available = IcOps::canister_cycle_balance().to_u128();
        let gateway_principal_count = BlobStorageLifecycleOps::gateway_principal_count();
        let last_gateway_principal_sync_at_ns =
            BlobStorageLifecycleOps::last_gateway_principal_sync_at_ns();

        let Some(config) = BlobStorageLifecycleOps::billing_config() else {
            let decision = decide_unconfigured_blob_storage_status(request.sync_gateway_principals);
            return Self::status_response(
                None,
                None,
                project_cycles_available,
                gateway_principal_count,
                last_gateway_principal_sync_at_ns,
                decision,
            );
        };

        let balance =
            Self::cashier_account_total_balance(config.cashier_canister_id, IcOps::canister_self())
                .await;
        let (cashier_balance, observation) = match balance {
            Ok(balance) => (
                Some(balance),
                BlobStorageCashierBalanceObservation::Available(balance),
            ),
            Err(BlobStorageBillingWorkflowError::CashierDecode(_)) => {
                (None, BlobStorageCashierBalanceObservation::Malformed)
            }
            Err(_) => (None, BlobStorageCashierBalanceObservation::Unavailable),
        };
        let decision = decide_configured_blob_storage_status(
            request.sync_gateway_principals,
            gateway_principal_count,
            observation,
            config.min_upload_balance,
            config.target_upload_balance,
            config.project_cycles_reserve,
            project_cycles_available,
        );

        Self::status_response(
            Some(config),
            cashier_balance,
            project_cycles_available,
            gateway_principal_count,
            last_gateway_principal_sync_at_ns,
            decision,
        )
    }

    /// Return the last successful gateway-authority synchronization timestamp.
    #[must_use]
    pub fn last_gateway_principal_sync_at_ns() -> Option<u64> {
        BlobStorageLifecycleOps::last_gateway_principal_sync_at_ns()
    }

    fn require_billing_config()
    -> Result<BlobStorageBillingConfigView, BlobStorageBillingWorkflowError> {
        BlobStorageLifecycleOps::billing_config()
            .ok_or(BlobStorageBillingWorkflowError::BillingConfigMissing)
    }

    fn top_up_report(
        requested_cycles: u128,
        attached_cycles: u128,
        project_cycles_before: u128,
        project_cycles_after: u128,
        reserve_cycles: u128,
        cashier_total_after: u128,
        skipped_reason: Option<String>,
    ) -> BlobProjectCyclesTopUpReport {
        BlobProjectCyclesTopUpReport {
            requested_cycles: BlobStorageConversionOps::billing_nat_from_u128(requested_cycles),
            attached_cycles: BlobStorageConversionOps::billing_nat_from_u128(attached_cycles),
            project_cycles_before: BlobStorageConversionOps::billing_nat_from_u128(
                project_cycles_before,
            ),
            project_cycles_after: BlobStorageConversionOps::billing_nat_from_u128(
                project_cycles_after,
            ),
            reserve_cycles: BlobStorageConversionOps::billing_nat_from_u128(reserve_cycles),
            cashier_total_after: BlobStorageConversionOps::billing_nat_from_u128(
                cashier_total_after,
            ),
            skipped_reason,
        }
    }

    fn status_response(
        config: Option<BlobStorageBillingConfigView>,
        cashier_balance: Option<u128>,
        project_cycles_available: u128,
        gateway_principal_count: u64,
        last_gateway_principal_sync_at_ns: Option<u64>,
        decision: BlobStorageStatusDecision,
    ) -> BlobStorageStatusResponse {
        BlobStorageStatusResponse {
            payment_model: if config.is_some() {
                BlobStoragePaymentModelStatus::ProjectAsPaymentAccount
            } else {
                BlobStoragePaymentModelStatus::NotConfigured
            },
            cashier_canister_id: config.map(|value| value.cashier_canister_id),
            payment_account: config.map(|_| IcOps::canister_self()),
            cashier_balance: cashier_balance.map(BlobStorageConversionOps::billing_nat_from_u128),
            min_upload_balance: config.map(|value| {
                BlobStorageConversionOps::billing_nat_from_u128(value.min_upload_balance)
            }),
            target_upload_balance: config.map(|value| {
                BlobStorageConversionOps::billing_nat_from_u128(value.target_upload_balance)
            }),
            project_cycles_reserve: config.map(|value| {
                BlobStorageConversionOps::billing_nat_from_u128(value.project_cycles_reserve)
            }),
            project_cycles_available: BlobStorageConversionOps::billing_nat_from_u128(
                project_cycles_available,
            ),
            gateway_principal_count,
            last_gateway_principal_sync_at_ns,
            gateway_principal_sync_action: decision.gateway_principal_sync_action,
            funding_status: Self::funding_status(decision.funding_status),
            ready: decision.ready,
            blockers: decision.blockers,
            warnings: decision.warnings,
        }
    }

    fn funding_status(decision: BlobStorageFundingStatusDecision) -> BlobStorageFundingStatus {
        match decision {
            BlobStorageFundingStatusDecision::NotConfigured => {
                BlobStorageFundingStatus::NotConfigured
            }
            BlobStorageFundingStatusDecision::NotNeeded => BlobStorageFundingStatus::NotNeeded,
            BlobStorageFundingStatusDecision::FundingRequired { requested_cycles } => {
                BlobStorageFundingStatus::FundingRequired {
                    requested_cycles: BlobStorageConversionOps::billing_nat_from_u128(
                        requested_cycles,
                    ),
                }
            }
            BlobStorageFundingStatusDecision::BalanceUnavailable => {
                BlobStorageFundingStatus::BalanceUnavailable
            }
            BlobStorageFundingStatusDecision::BalanceMalformed => {
                BlobStorageFundingStatus::BalanceMalformed
            }
            BlobStorageFundingStatusDecision::ReserveWouldBeViolated {
                requested_cycles,
                transferable_cycles,
            } => BlobStorageFundingStatus::ReserveWouldBeViolated {
                requested_cycles: BlobStorageConversionOps::billing_nat_from_u128(requested_cycles),
                transferable_cycles: BlobStorageConversionOps::billing_nat_from_u128(
                    transferable_cycles,
                ),
            },
        }
    }
}

/// Typed workflow failure retained until API projection.
#[derive(Debug, ThisError)]
pub enum BlobStorageBillingWorkflowError {
    #[error("blob-storage billing config is not set")]
    BillingConfigMissing,

    #[error(transparent)]
    BillingPolicy(#[from] BlobStorageBillingPolicyError),

    #[error(transparent)]
    BoundaryConversion(#[from] BlobStorageConversionError),

    #[error(transparent)]
    CashierDecode(#[from] CashierDecodeError),

    #[error("{0}")]
    CashierBalanceInternal(String),

    #[error("Cashier top-up rejected")]
    CashierTopUp(BlobStorageCashierAccountTopUpError),

    #[error(transparent)]
    FundingInProgress(#[from] BlobStorageFundingInProgress),

    #[error(transparent)]
    Internal(#[from] InternalError),
}
