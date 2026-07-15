//! Module: api::blob_storage::billing
//!
//! Responsibility: expose blob-storage billing helpers and map public errors.
//! Does not own: Cashier sequencing, funding policy, guards, or lifecycle storage.
//! Boundary: delegates immediately to the canonical billing workflow.

use super::BlobStorageApi;
use crate::{
    cdk::types::Principal,
    dto::{
        blob_storage::{
            BlobProjectCyclesTopUpReport, BlobStorageBillingConfig,
            BlobStorageCashierAccountBalanceGetResult, BlobStorageCashierAccountTopUpError,
            BlobStorageCashierAccountTopUpRequest, BlobStorageCashierAccountTopUpResult,
            BlobStorageStatusRequest, BlobStorageStatusResponse,
        },
        error::{Error, ErrorCode},
    },
    workflow::blob_storage::billing::{
        BlobStorageBillingWorkflow, BlobStorageBillingWorkflowError,
    },
};

impl BlobStorageApi {
    /// Store validated blob-storage billing configuration.
    pub fn configure_billing(config: BlobStorageBillingConfig) -> Result<(), Error> {
        BlobStorageBillingWorkflow::configure_billing(config).map_err(Self::map_billing_error)
    }

    /// Return the stored blob-storage billing configuration, if one is set.
    #[must_use]
    pub fn billing_config() -> Option<BlobStorageBillingConfig> {
        BlobStorageBillingWorkflow::billing_config()
    }

    /// Call Cashier `account_balance_get_v1` and return the typed raw result.
    pub async fn cashier_account_balance_get(
        cashier_canister_id: Principal,
        account: Principal,
    ) -> Result<BlobStorageCashierAccountBalanceGetResult, Error> {
        BlobStorageBillingWorkflow::cashier_account_balance_get(cashier_canister_id, account)
            .await
            .map_err(Self::map_billing_error)
    }

    /// Call Cashier `account_balance_get_v1` and decode the total balance.
    pub async fn cashier_account_total_balance(
        cashier_canister_id: Principal,
        account: Principal,
    ) -> Result<u128, Error> {
        BlobStorageBillingWorkflow::cashier_account_total_balance(cashier_canister_id, account)
            .await
            .map_err(Self::map_billing_error)
    }

    /// Call Cashier `account_top_up_v1` with the admitted cycle amount.
    pub async fn cashier_account_top_up(
        cashier_canister_id: Principal,
        request: Option<BlobStorageCashierAccountTopUpRequest>,
        cycles: u128,
    ) -> Result<BlobStorageCashierAccountTopUpResult, Error> {
        BlobStorageBillingWorkflow::cashier_account_top_up(cashier_canister_id, request, cycles)
            .await
            .map_err(Self::map_billing_error)
    }

    /// Sync Cashier gateway principals into local gateway authority.
    pub async fn sync_gateway_principals_from_cashier(
        cashier_canister_id: Principal,
        max_gateway_principals: usize,
    ) -> Result<u64, Error> {
        BlobStorageBillingWorkflow::sync_gateway_principals_from_cashier(
            cashier_canister_id,
            max_gateway_principals,
        )
        .await
        .map_err(Self::map_billing_error)
    }

    /// Sync gateway principals from the configured Cashier canister.
    pub async fn sync_gateway_principals_from_configured_cashier() -> Result<u64, Error> {
        BlobStorageBillingWorkflow::sync_gateway_principals_from_configured_cashier()
            .await
            .map_err(Self::map_billing_error)
    }

    /// Fund the configured Cashier account from this canister's cycles.
    pub async fn fund_from_project_cycles(
        requested_cycles: u128,
    ) -> Result<BlobProjectCyclesTopUpReport, Error> {
        BlobStorageBillingWorkflow::fund_from_project_cycles(requested_cycles)
            .await
            .map_err(Self::map_billing_error)
    }

    /// Return backend blob-storage billing status without transferring cycles.
    pub async fn status(request: BlobStorageStatusRequest) -> BlobStorageStatusResponse {
        BlobStorageBillingWorkflow::status(request).await
    }

    /// Return the last successful Cashier gateway-principal sync timestamp.
    #[must_use]
    pub fn last_gateway_principal_sync_at_ns() -> Option<u64> {
        BlobStorageBillingWorkflow::last_gateway_principal_sync_at_ns()
    }

    pub(super) fn map_billing_error(err: BlobStorageBillingWorkflowError) -> Error {
        match err {
            BlobStorageBillingWorkflowError::BillingConfigMissing
            | BlobStorageBillingWorkflowError::BillingPolicy(_)
            | BlobStorageBillingWorkflowError::BoundaryConversion(_) => {
                Error::invalid(err.to_string())
            }
            BlobStorageBillingWorkflowError::CashierDecode(err) => {
                Error::new(ErrorCode::InternalRpcMalformed, err.to_string())
            }
            BlobStorageBillingWorkflowError::CashierBalanceInternal(message) => {
                Error::internal(message)
            }
            BlobStorageBillingWorkflowError::CashierTopUp(err) => {
                Self::map_cashier_top_up_error(err)
            }
            BlobStorageBillingWorkflowError::FundingInProgress(err) => {
                Error::conflict(err.to_string())
            }
            BlobStorageBillingWorkflowError::Internal(err) => Error::from(err),
        }
    }

    pub(super) fn map_cashier_top_up_error(err: BlobStorageCashierAccountTopUpError) -> Error {
        match err {
            BlobStorageCashierAccountTopUpError::NotAuthorized(principal) => {
                Error::forbidden(format!("Cashier rejected top-up for account {principal}"))
            }
            BlobStorageCashierAccountTopUpError::AccountBalanceOverflow => {
                Error::exhausted("Cashier account balance overflow")
            }
            BlobStorageCashierAccountTopUpError::InternalError(message) => {
                Error::internal(format!("Cashier top-up failed: {message}"))
            }
            BlobStorageCashierAccountTopUpError::TopUpWithoutCycles => {
                Error::invalid("Cashier top-up rejected request without attached cycles")
            }
        }
    }
}
