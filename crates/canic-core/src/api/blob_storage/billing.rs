//! Module: api::blob_storage::billing
//!
//! Responsibility: expose blob-storage billing status, sync, and funding helpers.
//! Does not own: Cashier protocol DTOs, funding guards, or lifecycle storage.
//! Boundary: sequences configured billing ops and maps endpoint-facing errors.

use super::BlobStorageApi;
use crate::{
    cdk::{candid::Nat, types::Principal},
    dto::{
        blob_storage::{
            BlobProjectCyclesTopUpReport, BlobStorageBillingConfig, BlobStorageBillingWarning,
            BlobStorageCashierAccountBalanceGetError, BlobStorageCashierAccountBalanceGetResult,
            BlobStorageCashierAccountTopUpError, BlobStorageCashierAccountTopUpRequest,
            BlobStorageCashierAccountTopUpResult, BlobStorageFundingStatus,
            BlobStorageGatewayPrincipalSyncAction, BlobStoragePaymentModelStatus,
            BlobStorageReadinessBlocker, BlobStorageStatusRequest, BlobStorageStatusResponse,
        },
        error::{Error, ErrorCode},
    },
    ops::{
        blob_storage::{
            funding::{BlobStorageFundingInProgress, BlobStorageFundingOps},
            lifecycle::BlobStorageLifecycleOps,
        },
        cashier::{
            client::CashierClientOps,
            conversion::{CashierConversionOps, CashierDecodeError},
        },
        ic::{IcOps, mgmt::MgmtOps},
    },
};

impl BlobStorageApi {
    /// Store validated blob-storage billing configuration.
    pub fn configure_billing(config: BlobStorageBillingConfig) -> Result<(), Error> {
        if config.cashier_canister_id == Principal::anonymous()
            || config.cashier_canister_id == Principal::management_canister()
        {
            return Err(Error::invalid(
                "cashier_canister_id must be a concrete canister principal",
            ));
        }
        if config.gateway_principal_limit == 0 {
            return Err(Error::invalid(
                "gateway_principal_limit must be greater than zero",
            ));
        }
        let _gateway_principal_limit = usize::try_from(config.gateway_principal_limit)
            .map_err(|_| Error::invalid("gateway_principal_limit exceeds usize"))?;

        let project_cycles_reserve =
            Self::nat_to_u128("project_cycles_reserve", &config.project_cycles_reserve)?;
        let min_upload_balance =
            Self::nat_to_u128("min_upload_balance", &config.min_upload_balance)?;
        let target_upload_balance =
            Self::nat_to_u128("target_upload_balance", &config.target_upload_balance)?;

        if project_cycles_reserve == 0 {
            return Err(Error::invalid(
                "project_cycles_reserve must be greater than zero",
            ));
        }
        if min_upload_balance == 0 {
            return Err(Error::invalid(
                "min_upload_balance must be greater than zero",
            ));
        }
        if target_upload_balance == 0 {
            return Err(Error::invalid(
                "target_upload_balance must be greater than zero",
            ));
        }
        if min_upload_balance > target_upload_balance {
            return Err(Error::invalid(
                "min_upload_balance must be less than or equal to target_upload_balance",
            ));
        }

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

    /// Return the stored blob-storage billing configuration, if one is set.
    #[must_use]
    pub fn billing_config() -> Option<BlobStorageBillingConfig> {
        BlobStorageLifecycleOps::billing_config_dto()
    }

    /// Call Cashier `account_balance_get_v1` and return the typed raw result.
    pub async fn cashier_account_balance_get(
        cashier_canister_id: Principal,
        account: Principal,
    ) -> Result<BlobStorageCashierAccountBalanceGetResult, Error> {
        CashierClientOps::account_balance_get(cashier_canister_id, account)
            .await
            .map_err(Error::from)
    }

    /// Call Cashier `account_balance_get_v1` and convert the total balance to `u128`.
    pub async fn cashier_account_total_balance(
        cashier_canister_id: Principal,
        account: Principal,
    ) -> Result<u128, Error> {
        match Self::cashier_account_balance_get(cashier_canister_id, account).await? {
            BlobStorageCashierAccountBalanceGetResult::Ok(balance) => {
                CashierConversionOps::account_cycle_balances_to_u128(
                    &balance.account_cycle_balances,
                )
                .map(|balances| balances.total)
                .map_err(Self::map_cashier_decode_error)
            }
            BlobStorageCashierAccountBalanceGetResult::Err(
                BlobStorageCashierAccountBalanceGetError::AccountNotFound,
            ) => Ok(0),
            BlobStorageCashierAccountBalanceGetResult::Err(
                BlobStorageCashierAccountBalanceGetError::InternalError(message),
            ) => Err(Error::internal(message)),
        }
    }

    /// Call Cashier `account_top_up_v1` with the already-approved cycle amount.
    pub async fn cashier_account_top_up(
        cashier_canister_id: Principal,
        request: Option<BlobStorageCashierAccountTopUpRequest>,
        cycles: u128,
    ) -> Result<BlobStorageCashierAccountTopUpResult, Error> {
        CashierClientOps::account_top_up(cashier_canister_id, request, cycles)
            .await
            .map_err(Error::from)
    }

    /// Sync Cashier gateway principals into the local blob-storage gateway store.
    pub async fn sync_gateway_principals_from_cashier(
        cashier_canister_id: Principal,
        max_gateway_principals: usize,
    ) -> Result<u64, Error> {
        let principals = CashierClientOps::storage_gateway_principal_list(cashier_canister_id)
            .await
            .map_err(Error::from)?;
        let principals =
            CashierConversionOps::normalize_gateway_principals(principals, max_gateway_principals)
                .map_err(Self::map_cashier_decode_error)?;

        let now_ns = IcOps::now_nanos();
        let count = BlobStorageLifecycleOps::replace_gateway_principals(&principals, now_ns);
        BlobStorageLifecycleOps::record_gateway_principal_sync(now_ns);
        Ok(count)
    }

    /// Sync gateway principals from the configured Cashier canister.
    pub async fn sync_gateway_principals_from_configured_cashier() -> Result<u64, Error> {
        let Some(config) = BlobStorageLifecycleOps::billing_config() else {
            return Err(Error::invalid("blob-storage billing config is not set"));
        };
        let max_gateway_principals = usize::try_from(config.gateway_principal_limit)
            .map_err(|_| Error::invalid("gateway_principal_limit exceeds usize"))?;

        Self::sync_gateway_principals_from_cashier(
            config.cashier_canister_id,
            max_gateway_principals,
        )
        .await
    }

    /// Fund the configured Cashier account from this canister's cycles.
    pub async fn fund_from_project_cycles(
        requested_cycles: u128,
    ) -> Result<BlobProjectCyclesTopUpReport, Error> {
        let Some(config) = BlobStorageLifecycleOps::billing_config() else {
            return Err(Error::invalid("blob-storage billing config is not set"));
        };
        Self::validate_requested_funding_cycles(requested_cycles)?;
        let _funding_guard =
            BlobStorageFundingOps::try_acquire().map_err(Self::map_funding_in_progress)?;

        let attachment = Self::funding_attachment(
            requested_cycles,
            MgmtOps::canister_cycle_balance().to_u128(),
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
                return Err(Self::map_cashier_top_up_error(err));
            }
        };
        let cashier_total_after =
            CashierConversionOps::account_cycle_balances_to_u128(&top_up.balance)
                .map(|balances| balances.total)
                .map_err(Self::map_cashier_decode_error)?;
        let project_cycles_after = MgmtOps::canister_cycle_balance().to_u128();

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

    /// Return backend blob-storage billing status without transferring cycles.
    pub async fn status(request: BlobStorageStatusRequest) -> BlobStorageStatusResponse {
        let project_cycles_available = MgmtOps::canister_cycle_balance().to_u128();
        let gateway_principal_count = Self::gateway_principal_count();
        let last_gateway_principal_sync_at_ns =
            BlobStorageLifecycleOps::last_gateway_principal_sync_at_ns();

        let Some(config) = BlobStorageLifecycleOps::billing_config() else {
            return BlobStorageStatusResponse {
                payment_model: BlobStoragePaymentModelStatus::NotConfigured,
                cashier_canister_id: None,
                payment_account: None,
                cashier_balance: None,
                min_upload_balance: None,
                target_upload_balance: None,
                project_cycles_reserve: None,
                project_cycles_available: Self::nat_from_u128(project_cycles_available),
                gateway_principal_count,
                last_gateway_principal_sync_at_ns,
                gateway_principal_sync_action: Self::status_sync_action(&request, false),
                funding_status: BlobStorageFundingStatus::NotConfigured,
                ready: false,
                blockers: vec![BlobStorageReadinessBlocker::NotConfigured],
                warnings: Vec::new(),
            };
        };

        let mut blockers = Vec::new();
        let mut warnings = Vec::new();
        if request.sync_gateway_principals {
            warnings.push(BlobStorageBillingWarning::SyncRequestedButStatusIsReadOnly);
        }
        if gateway_principal_count == 0 {
            blockers.push(BlobStorageReadinessBlocker::GatewayPrincipalsMissing);
            warnings.push(BlobStorageBillingWarning::GatewayPrincipalSetEmpty);
        }

        let balance =
            Self::cashier_account_total_balance(config.cashier_canister_id, IcOps::canister_self())
                .await;
        let (cashier_balance, funding_status) = match balance {
            Ok(balance) => {
                let funding_status = Self::status_funding_status(
                    balance,
                    config.min_upload_balance,
                    config.target_upload_balance,
                    config.project_cycles_reserve,
                    project_cycles_available,
                    &mut blockers,
                );
                (Some(Self::nat_from_u128(balance)), funding_status)
            }
            Err(err) if err.code == ErrorCode::InternalRpcMalformed => {
                blockers.push(BlobStorageReadinessBlocker::CashierBalanceMalformed);
                warnings.push(BlobStorageBillingWarning::CashierBalanceMalformed);
                (None, BlobStorageFundingStatus::BalanceMalformed)
            }
            Err(_) => {
                blockers.push(BlobStorageReadinessBlocker::CashierBalanceUnavailable);
                warnings.push(BlobStorageBillingWarning::CashierBalanceUnavailable);
                (None, BlobStorageFundingStatus::BalanceUnavailable)
            }
        };

        BlobStorageStatusResponse {
            payment_model: BlobStoragePaymentModelStatus::ProjectAsPaymentAccount,
            cashier_canister_id: Some(config.cashier_canister_id),
            payment_account: Some(IcOps::canister_self()),
            cashier_balance,
            min_upload_balance: Some(Self::nat_from_u128(config.min_upload_balance)),
            target_upload_balance: Some(Self::nat_from_u128(config.target_upload_balance)),
            project_cycles_reserve: Some(Self::nat_from_u128(config.project_cycles_reserve)),
            project_cycles_available: Self::nat_from_u128(project_cycles_available),
            gateway_principal_count,
            last_gateway_principal_sync_at_ns,
            gateway_principal_sync_action: Self::status_sync_action(&request, true),
            funding_status,
            ready: blockers.is_empty(),
            blockers,
            warnings,
        }
    }

    /// Return the last successful Cashier gateway-principal sync timestamp.
    #[must_use]
    pub fn last_gateway_principal_sync_at_ns() -> Option<u64> {
        BlobStorageLifecycleOps::last_gateway_principal_sync_at_ns()
    }

    pub(super) fn map_cashier_decode_error(err: CashierDecodeError) -> Error {
        Error::new(ErrorCode::InternalRpcMalformed, err.to_string())
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

    pub(super) fn map_funding_in_progress(err: BlobStorageFundingInProgress) -> Error {
        Error::conflict(err.to_string())
    }

    fn nat_to_u128(field: &str, value: &Nat) -> Result<u128, Error> {
        u128::try_from(value.0.clone()).map_err(|_| Error::invalid(format!("{field} exceeds u128")))
    }

    pub(super) fn nat_from_u128(value: u128) -> Nat {
        Nat::parse(value.to_string().as_bytes()).expect("u128 must encode as Candid nat")
    }

    pub(super) fn validate_requested_funding_cycles(requested_cycles: u128) -> Result<(), Error> {
        if requested_cycles == 0 {
            return Err(Error::invalid("requested_cycles must be greater than zero"));
        }

        Ok(())
    }

    pub(super) const fn funding_attachment(
        requested_cycles: u128,
        project_cycles_available: u128,
        project_cycles_reserve: u128,
    ) -> BlobStorageFundingAttachment {
        let transferable_cycles = project_cycles_available.saturating_sub(project_cycles_reserve);
        let reserve_would_be_violated = requested_cycles > transferable_cycles;
        let attached_cycles = if reserve_would_be_violated {
            0
        } else {
            requested_cycles
        };
        let skipped_reason = if reserve_would_be_violated {
            Some("reserve would be violated")
        } else {
            None
        };

        BlobStorageFundingAttachment {
            project_cycles_available,
            attached_cycles,
            skipped_reason,
        }
    }

    pub(super) const fn status_sync_action(
        request: &BlobStorageStatusRequest,
        has_config: bool,
    ) -> BlobStorageGatewayPrincipalSyncAction {
        if !request.sync_gateway_principals {
            return BlobStorageGatewayPrincipalSyncAction::NotRequested;
        }
        if has_config {
            BlobStorageGatewayPrincipalSyncAction::SkippedReadOnlyStatus
        } else {
            BlobStorageGatewayPrincipalSyncAction::SkippedConfigMissing
        }
    }

    pub(super) fn status_funding_status(
        cashier_balance: u128,
        min_upload_balance: u128,
        target_upload_balance: u128,
        project_cycles_reserve: u128,
        project_cycles_available: u128,
        blockers: &mut Vec<BlobStorageReadinessBlocker>,
    ) -> BlobStorageFundingStatus {
        if cashier_balance >= min_upload_balance {
            return BlobStorageFundingStatus::NotNeeded;
        }

        blockers.push(BlobStorageReadinessBlocker::InsufficientCashierBalance);
        let requested_cycles = target_upload_balance.saturating_sub(cashier_balance);
        let transferable_cycles = project_cycles_available.saturating_sub(project_cycles_reserve);
        if requested_cycles > transferable_cycles {
            blockers.push(BlobStorageReadinessBlocker::ReserveWouldBeViolated);
            return BlobStorageFundingStatus::ReserveWouldBeViolated {
                requested_cycles: Self::nat_from_u128(requested_cycles),
                transferable_cycles: Self::nat_from_u128(transferable_cycles),
            };
        }

        BlobStorageFundingStatus::FundingRequired {
            requested_cycles: Self::nat_from_u128(requested_cycles),
        }
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
            requested_cycles: Self::nat_from_u128(requested_cycles),
            attached_cycles: Self::nat_from_u128(attached_cycles),
            project_cycles_before: Self::nat_from_u128(project_cycles_before),
            project_cycles_after: Self::nat_from_u128(project_cycles_after),
            reserve_cycles: Self::nat_from_u128(reserve_cycles),
            cashier_total_after: Self::nat_from_u128(cashier_total_after),
            skipped_reason,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct BlobStorageFundingAttachment {
    pub(super) project_cycles_available: u128,
    pub(super) attached_cycles: u128,
    pub(super) skipped_reason: Option<&'static str>,
}
