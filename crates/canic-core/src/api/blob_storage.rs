//! Module: api::blob_storage
//!
//! Responsibility: expose blob-storage helpers used by macro-generated endpoints.
//! Does not own: stable storage, gateway authorization, or lifecycle workflows.
//! Boundary: maps public endpoint inputs into ops/model validation and public errors.

use crate::{
    cdk::types::Principal,
    dto::{
        blob_storage::{BlobStorageLocalCounters, CreateCertificateResult},
        error::Error,
    },
    ops::{
        blob_storage::{
            conversion::{BlobStorageConversionError, BlobStorageConversionOps},
            lifecycle::{
                BlobPendingDeletionOutcome, BlobRegisterOutcome, BlobStorageLifecycleError,
                BlobStorageLifecycleOps,
            },
        },
        ic::IcOps,
    },
};

#[cfg(feature = "blob-storage-billing")]
use crate::{
    InternalError,
    cdk::candid::Nat,
    dto::blob_storage::{
        BlobProjectCyclesTopUpReport, BlobStorageBillingConfig, BlobStorageBillingWarning,
        BlobStorageCashierAccountBalanceGetError, BlobStorageCashierAccountBalanceGetResult,
        BlobStorageCashierAccountTopUpRequest, BlobStorageCashierAccountTopUpResult,
        BlobStorageFundingStatus, BlobStorageGatewayPrincipalSyncAction,
        BlobStoragePaymentModelStatus, BlobStorageReadinessBlocker, BlobStorageStatusRequest,
        BlobStorageStatusResponse,
    },
    ops::{
        cashier::{
            client::CashierClientOps,
            conversion::{CashierConversionOps, CashierDecodeError},
        },
        ic::mgmt::MgmtOps,
    },
    storage::stable::blob_storage::BlobStorageBillingConfigRecord,
};

///
/// BlobStorageApi
///
/// Public facade for feature-gated blob-storage endpoint helpers.
///

pub struct BlobStorageApi;

impl BlobStorageApi {
    /// Store validated blob-storage billing configuration.
    #[cfg(feature = "blob-storage-billing")]
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
        if min_upload_balance > target_upload_balance {
            return Err(Error::invalid(
                "min_upload_balance must be less than or equal to target_upload_balance",
            ));
        }

        BlobStorageLifecycleOps::set_billing_config(BlobStorageBillingConfigRecord::new(
            config.cashier_canister_id,
            project_cycles_reserve,
            min_upload_balance,
            target_upload_balance,
            config.gateway_principal_limit,
            IcOps::now_nanos(),
        ));
        Ok(())
    }

    /// Return the stored blob-storage billing configuration, if one is set.
    #[cfg(feature = "blob-storage-billing")]
    pub fn billing_config() -> Option<BlobStorageBillingConfig> {
        BlobStorageLifecycleOps::billing_config().map(Self::billing_config_record_to_dto)
    }

    /// Canonicalize a Toko/Caffeine root hash string into `sha256:<64-lowercase-hex>`.
    pub fn canonical_root_hash_text(value: &str) -> Result<String, Error> {
        BlobStorageConversionOps::root_hash_from_text(value)
            .map(crate::model::blob_storage::BlobRootHash::into_string)
            .map_err(Self::map_conversion_error)
    }

    /// Canonicalize a gateway 32-byte root hash into `sha256:<64-lowercase-hex>`.
    pub fn canonical_root_hash_bytes(bytes: &[u8]) -> Result<String, Error> {
        BlobStorageConversionOps::root_hash_from_bytes(bytes)
            .map(crate::model::blob_storage::BlobRootHash::into_string)
            .map_err(Self::map_conversion_error)
    }

    /// Register a live blob root. Returns `true` when new live state was inserted.
    pub fn register_live(root_hash: &str, now_ns: u64) -> Result<bool, Error> {
        let hash = BlobStorageConversionOps::root_hash_from_text(root_hash)
            .map_err(Self::map_conversion_error)?;
        BlobStorageLifecycleOps::register_live(&hash, now_ns)
            .map(BlobRegisterOutcome::inserted)
            .map_err(Self::map_lifecycle_error)
    }

    /// Register an upload certificate request and return the gateway-compatible DTO.
    ///
    /// The gateway contract echoes the request hash in the response; Canic stores
    /// the canonical normalized hash internally.
    pub fn create_certificate(root_hash: String) -> Result<CreateCertificateResult, Error> {
        let hash = BlobStorageConversionOps::root_hash_from_text(&root_hash)
            .map_err(Self::map_conversion_error)?;
        BlobStorageLifecycleOps::register_live(&hash, IcOps::now_nanos())
            .map_err(Self::map_lifecycle_error)?;

        Ok(CreateCertificateResult {
            method: "upload".to_string(),
            blob_hash: root_hash,
        })
    }

    /// Evaluate gateway liveness query inputs, returning `false` for malformed byte entries.
    #[must_use]
    pub fn blobs_are_live(hash_bytes_list: Vec<Vec<u8>>) -> Vec<bool> {
        hash_bytes_list
            .iter()
            .map(|bytes| {
                let Ok(hash) = BlobStorageConversionOps::root_hash_from_bytes(bytes) else {
                    return false;
                };
                BlobStorageLifecycleOps::is_live(&hash)
            })
            .collect()
    }

    /// Return whether a blob root is registered live and not pending deletion.
    pub fn is_live(root_hash: &str) -> Result<bool, Error> {
        let hash = BlobStorageConversionOps::root_hash_from_text(root_hash)
            .map_err(Self::map_conversion_error)?;
        Ok(BlobStorageLifecycleOps::is_live(&hash))
    }

    /// Require a live blob root, returning `NotFound` when it is missing or pending deletion.
    pub fn require_live(root_hash: &str) -> Result<(), Error> {
        let hash = BlobStorageConversionOps::root_hash_from_text(root_hash)
            .map_err(Self::map_conversion_error)?;
        BlobStorageLifecycleOps::require_live(&hash)
            .map(|_| ())
            .map_err(Self::map_lifecycle_error)
    }

    /// Mark a live blob as pending gateway deletion.
    pub fn mark_pending_delete(root_hash: &str, now_ns: u64) -> Result<bool, Error> {
        let hash = BlobStorageConversionOps::root_hash_from_text(root_hash)
            .map_err(Self::map_conversion_error)?;
        BlobStorageLifecycleOps::mark_pending_delete(&hash, now_ns)
            .map(BlobPendingDeletionOutcome::inserted)
            .map_err(Self::map_lifecycle_error)
    }

    /// Confirm gateway deletion from the gateway's 32-byte root hash input.
    pub fn confirm_deleted_by_gateway_hash_bytes(bytes: &[u8]) -> Result<(), Error> {
        let hash = BlobStorageConversionOps::root_hash_from_bytes(bytes)
            .map_err(Self::map_conversion_error)?;
        BlobStorageLifecycleOps::confirm_deleted_by_gateway(&hash);
        Ok(())
    }

    /// Return the number of stored blob records, including pending-deletion records.
    #[must_use]
    pub fn stored_blob_count() -> u64 {
        BlobStorageLifecycleOps::stored_blob_count()
    }

    /// Return the number of pending gateway-deletion records.
    #[must_use]
    pub fn pending_deletion_count() -> u64 {
        BlobStorageLifecycleOps::pending_deletion_count()
    }

    /// Return local operational counters for host-owned guarded status endpoints.
    #[must_use]
    pub fn local_counters() -> BlobStorageLocalCounters {
        BlobStorageLocalCounters::new(
            Self::stored_blob_count(),
            Self::pending_deletion_count(),
            Self::gateway_principal_count(),
        )
    }

    /// Return pending-deletion root hashes in stable key order.
    #[must_use]
    pub fn pending_deletion_hashes() -> Vec<String> {
        BlobStorageLifecycleOps::pending_deletion_hashes()
    }

    /// Return pending-deletion roots only to registered storage gateways.
    #[must_use]
    pub fn pending_deletion_hashes_for_gateway(caller: Principal) -> Vec<String> {
        if !BlobStorageLifecycleOps::is_gateway_principal(caller) {
            return Vec::new();
        }
        BlobStorageLifecycleOps::pending_deletion_hashes()
    }

    /// Confirm gateway deletion for valid 32-byte roots when caller is a registered gateway.
    pub fn confirm_deleted_by_gateway_hash_bytes_batch(
        caller: Principal,
        hash_bytes_list: Vec<Vec<u8>>,
    ) {
        if !BlobStorageLifecycleOps::is_gateway_principal(caller) {
            return;
        }

        for bytes in &hash_bytes_list {
            if let Ok(hash) = BlobStorageConversionOps::root_hash_from_bytes(bytes) {
                BlobStorageLifecycleOps::confirm_deleted_by_gateway(&hash);
            }
        }
    }

    /// Insert or update an authorized storage gateway principal.
    pub fn upsert_gateway_principal(principal: Principal, now_ns: u64) {
        BlobStorageLifecycleOps::upsert_gateway_principal(principal, now_ns);
    }

    /// Replace authorized storage gateway principals.
    #[must_use]
    pub fn replace_gateway_principals(principals: &[Principal], now_ns: u64) -> u64 {
        BlobStorageLifecycleOps::replace_gateway_principals(principals, now_ns)
    }

    /// Remove an authorized storage gateway principal.
    #[must_use]
    pub fn remove_gateway_principal(principal: Principal) -> bool {
        BlobStorageLifecycleOps::remove_gateway_principal(principal)
    }

    /// Return the number of authorized storage gateway principals.
    #[must_use]
    pub fn gateway_principal_count() -> u64 {
        BlobStorageLifecycleOps::gateway_principal_count()
    }

    /// Return whether the principal is an authorized storage gateway.
    #[must_use]
    pub fn is_gateway_principal(principal: Principal) -> bool {
        BlobStorageLifecycleOps::is_gateway_principal(principal)
    }

    /// Call Cashier `account_balance_get_v1` and return the typed raw result.
    #[cfg(feature = "blob-storage-billing")]
    pub async fn cashier_account_balance_get(
        cashier_canister_id: Principal,
        account: Principal,
    ) -> Result<BlobStorageCashierAccountBalanceGetResult, Error> {
        CashierClientOps::account_balance_get(cashier_canister_id, account)
            .await
            .map_err(Error::from)
    }

    /// Call Cashier `account_balance_get_v1` and convert the total balance to `u128`.
    #[cfg(feature = "blob-storage-billing")]
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
    #[cfg(feature = "blob-storage-billing")]
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
    #[cfg(feature = "blob-storage-billing")]
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
    #[cfg(feature = "blob-storage-billing")]
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
    #[cfg(feature = "blob-storage-billing")]
    pub async fn fund_from_project_cycles(
        requested_cycles: u128,
    ) -> Result<BlobProjectCyclesTopUpReport, Error> {
        let Some(config) = BlobStorageLifecycleOps::billing_config() else {
            return Err(Error::invalid("blob-storage billing config is not set"));
        };

        let project_cycles_before = MgmtOps::canister_cycle_balance().to_u128();
        let transferable = project_cycles_before.saturating_sub(config.project_cycles_reserve);
        let attached_cycles = requested_cycles.min(transferable);

        if attached_cycles == 0 {
            return Ok(Self::top_up_report(
                requested_cycles,
                0,
                project_cycles_before,
                project_cycles_before,
                config.project_cycles_reserve,
                0,
                Some("reserve would be violated".to_string()),
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
        let BlobStorageCashierAccountTopUpResult::Ok(top_up) = result else {
            return Err(Error::internal("Cashier top-up failed"));
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
    #[cfg(feature = "blob-storage-billing")]
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
        let (cashier_balance, funding_status) = if let Ok(balance) = balance {
            let funding_status = Self::status_funding_status(
                balance,
                config.min_upload_balance,
                config.target_upload_balance,
                config.project_cycles_reserve,
                project_cycles_available,
                &mut blockers,
            );
            (Some(Self::nat_from_u128(balance)), funding_status)
        } else {
            blockers.push(BlobStorageReadinessBlocker::CashierBalanceUnavailable);
            warnings.push(BlobStorageBillingWarning::CashierBalanceUnavailable);
            (None, BlobStorageFundingStatus::BalanceUnavailable)
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
    #[cfg(feature = "blob-storage-billing")]
    #[must_use]
    pub fn last_gateway_principal_sync_at_ns() -> Option<u64> {
        BlobStorageLifecycleOps::last_gateway_principal_sync_at_ns()
    }

    fn map_conversion_error(err: BlobStorageConversionError) -> Error {
        Error::invalid(err.to_string())
    }

    fn map_lifecycle_error(err: BlobStorageLifecycleError) -> Error {
        match err {
            BlobStorageLifecycleError::BlobNotLive => Error::not_found(err.to_string()),
            BlobStorageLifecycleError::BlobPendingDeletion => Error::conflict(err.to_string()),
        }
    }

    #[cfg(feature = "blob-storage-billing")]
    fn map_cashier_decode_error(err: CashierDecodeError) -> Error {
        Error::from(InternalError::from(err))
    }

    #[cfg(feature = "blob-storage-billing")]
    fn nat_to_u128(field: &str, value: &Nat) -> Result<u128, Error> {
        u128::try_from(value.0.clone()).map_err(|_| Error::invalid(format!("{field} exceeds u128")))
    }

    #[cfg(feature = "blob-storage-billing")]
    fn billing_config_record_to_dto(
        record: BlobStorageBillingConfigRecord,
    ) -> BlobStorageBillingConfig {
        BlobStorageBillingConfig {
            cashier_canister_id: record.cashier_canister_id,
            project_cycles_reserve: Self::nat_from_u128(record.project_cycles_reserve),
            min_upload_balance: Self::nat_from_u128(record.min_upload_balance),
            target_upload_balance: Self::nat_from_u128(record.target_upload_balance),
            gateway_principal_limit: record.gateway_principal_limit,
        }
    }

    #[cfg(feature = "blob-storage-billing")]
    fn nat_from_u128(value: u128) -> Nat {
        Nat::parse(value.to_string().as_bytes()).expect("u128 must encode as Candid nat")
    }

    #[cfg(feature = "blob-storage-billing")]
    const fn status_sync_action(
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

    #[cfg(feature = "blob-storage-billing")]
    fn status_funding_status(
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

    #[cfg(feature = "blob-storage-billing")]
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::error::ErrorCode;

    #[test]
    fn canonical_root_hash_text_normalizes_toko_hashes() {
        let hash = BlobStorageApi::canonical_root_hash_text(
            "sha256:ABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCD",
        )
        .expect("hash parses");

        assert_eq!(
            hash,
            "sha256:abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
        );
    }

    #[test]
    fn canonical_root_hash_bytes_matches_gateway_query_shape() {
        let hash =
            BlobStorageApi::canonical_root_hash_bytes(&[0xabu8; 32]).expect("hash bytes convert");

        assert_eq!(
            hash,
            "sha256:abababababababababababababababababababababababababababababababab"
        );
    }

    #[test]
    fn malformed_root_hash_maps_to_public_invalid_input() {
        let err = BlobStorageApi::canonical_root_hash_text("sha256:zz")
            .expect_err("short malformed hash should fail");

        assert_eq!(err.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn create_certificate_echoes_request_hash_and_registers_canonical_root() {
        crate::storage::stable::blob_storage::BlobStorageStore::clear();
        let request_hash =
            "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string();
        let canonical_hash =
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

        let result = BlobStorageApi::create_certificate(request_hash.clone())
            .expect("create certificate succeeds");

        assert_eq!(
            result,
            CreateCertificateResult {
                method: "upload".to_string(),
                blob_hash: request_hash
            }
        );
        assert!(BlobStorageApi::is_live(canonical_hash).expect("canonical live check"));
        assert_eq!(
            BlobStorageApi::blobs_are_live(vec![vec![0xaau8; 32]]),
            vec![true]
        );
    }

    #[test]
    fn repeated_create_certificate_is_canonical_idempotent() {
        crate::storage::stable::blob_storage::BlobStorageStore::clear();
        let upper =
            "sha256:BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB".to_string();
        let lower =
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string();

        let first = BlobStorageApi::create_certificate(upper.clone()).expect("first create");
        let second = BlobStorageApi::create_certificate(lower.clone()).expect("second create");

        assert_eq!(first.blob_hash, upper);
        assert_eq!(second.blob_hash, lower);
        assert_eq!(BlobStorageApi::stored_blob_count(), 1);
        assert_eq!(BlobStorageApi::pending_deletion_count(), 0);
        assert!(BlobStorageApi::is_live(&lower).expect("canonical live check"));
    }

    #[test]
    fn malformed_api_inputs_do_not_mutate_blob_state() {
        crate::storage::stable::blob_storage::BlobStorageStore::clear();
        let malformed = "sha256:zz";

        assert_eq!(
            BlobStorageApi::create_certificate(malformed.to_string())
                .expect_err("malformed create fails")
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            BlobStorageApi::register_live(malformed, 10)
                .expect_err("malformed register fails")
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            BlobStorageApi::is_live(malformed)
                .expect_err("malformed live check fails")
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            BlobStorageApi::mark_pending_delete(malformed, 20)
                .expect_err("malformed pending delete fails")
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            BlobStorageApi::confirm_deleted_by_gateway_hash_bytes(&[0u8; 31])
                .expect_err("malformed gateway confirm fails")
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            BlobStorageApi::local_counters(),
            BlobStorageLocalCounters::new(0, 0, 0)
        );
        assert!(BlobStorageApi::pending_deletion_hashes().is_empty());
    }

    #[test]
    fn live_blob_lifecycle_maps_to_public_api() {
        crate::storage::stable::blob_storage::BlobStorageStore::clear();
        let hash = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

        assert!(!BlobStorageApi::is_live(hash).expect("live check"));
        assert_eq!(BlobStorageApi::stored_blob_count(), 0);
        assert_eq!(BlobStorageApi::pending_deletion_count(), 0);
        assert_eq!(
            BlobStorageApi::require_live(hash)
                .expect_err("missing blob is not live")
                .code,
            ErrorCode::NotFound
        );
        assert!(BlobStorageApi::register_live(hash, 10).expect("register"));
        assert!(!BlobStorageApi::register_live(hash, 20).expect("register again"));
        assert!(BlobStorageApi::is_live(hash).expect("live check"));
        assert_eq!(BlobStorageApi::stored_blob_count(), 1);
        assert_eq!(BlobStorageApi::pending_deletion_count(), 0);
        BlobStorageApi::require_live(hash).expect("require live");

        assert!(BlobStorageApi::mark_pending_delete(hash, 30).expect("mark pending"));
        assert!(!BlobStorageApi::mark_pending_delete(hash, 40).expect("mark again"));
        assert_eq!(BlobStorageApi::stored_blob_count(), 1);
        assert_eq!(BlobStorageApi::pending_deletion_count(), 1);
        assert_eq!(
            BlobStorageApi::local_counters(),
            BlobStorageLocalCounters::new(1, 1, 0)
        );
        assert_eq!(
            BlobStorageApi::require_live(hash)
                .expect_err("pending is not live")
                .code,
            ErrorCode::Conflict
        );
    }

    #[test]
    fn gateway_byte_confirmation_removes_live_blob() {
        crate::storage::stable::blob_storage::BlobStorageStore::clear();
        let hash = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
        let bytes = [0xddu8; 32];

        BlobStorageApi::register_live(hash, 10).expect("register");
        BlobStorageApi::mark_pending_delete(hash, 20).expect("mark pending");
        assert_eq!(
            BlobStorageApi::pending_deletion_hashes(),
            vec![hash.to_string()]
        );

        BlobStorageApi::confirm_deleted_by_gateway_hash_bytes(&bytes).expect("confirm");

        assert!(!BlobStorageApi::is_live(hash).expect("live check"));
        assert!(BlobStorageApi::pending_deletion_hashes().is_empty());
    }

    #[test]
    fn gateway_principal_api_is_idempotent() {
        let principal = Principal::from_slice(&[99; 29]);

        crate::storage::stable::blob_storage::BlobStorageStore::clear();
        assert!(!BlobStorageApi::is_gateway_principal(principal));
        assert_eq!(BlobStorageApi::gateway_principal_count(), 0);

        BlobStorageApi::upsert_gateway_principal(principal, 10);
        assert!(BlobStorageApi::is_gateway_principal(principal));
        assert_eq!(BlobStorageApi::gateway_principal_count(), 1);
        assert_eq!(
            BlobStorageApi::local_counters(),
            BlobStorageLocalCounters::new(0, 0, 1)
        );
        assert!(BlobStorageApi::remove_gateway_principal(principal));
        assert!(!BlobStorageApi::remove_gateway_principal(principal));
        assert_eq!(BlobStorageApi::gateway_principal_count(), 0);
    }

    #[test]
    fn gateway_endpoint_helpers_match_toko_malformed_input_behavior() {
        crate::storage::stable::blob_storage::BlobStorageStore::clear();
        let hash = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
        let bytes = [0xeeu8; 32];
        let gateway = Principal::from_slice(&[11; 29]);

        assert_eq!(
            BlobStorageApi::blobs_are_live(vec![bytes.to_vec(), vec![1, 2, 3]]),
            vec![false, false]
        );

        BlobStorageApi::create_certificate(hash.to_string()).expect("create certificate");
        assert_eq!(
            BlobStorageApi::blobs_are_live(vec![bytes.to_vec(), vec![1, 2, 3]]),
            vec![true, false]
        );

        BlobStorageApi::mark_pending_delete(hash, 10).expect("mark pending");
        assert!(BlobStorageApi::pending_deletion_hashes_for_gateway(gateway).is_empty());
        BlobStorageApi::confirm_deleted_by_gateway_hash_bytes_batch(gateway, vec![bytes.to_vec()]);
        assert_eq!(
            BlobStorageApi::pending_deletion_hashes(),
            vec![hash.to_string()]
        );

        BlobStorageApi::upsert_gateway_principal(gateway, 20);
        assert_eq!(
            BlobStorageApi::pending_deletion_hashes_for_gateway(gateway),
            vec![hash.to_string()]
        );

        BlobStorageApi::confirm_deleted_by_gateway_hash_bytes_batch(
            gateway,
            vec![vec![1, 2, 3], bytes.to_vec()],
        );

        assert!(BlobStorageApi::pending_deletion_hashes().is_empty());
        assert!(!BlobStorageApi::is_live(hash).expect("live check"));
    }
}
