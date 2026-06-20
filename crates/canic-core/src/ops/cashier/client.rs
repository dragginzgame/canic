//! Module: ops::cashier::client
//!
//! Responsibility: perform typed bounded-wait calls to the Cashier canister.
//! Does not own: billing policy, endpoint authorization, or production defaults.
//! Boundary: workflow supplies validated Cashier principals and funding decisions.

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::blob_storage::{
        BlobStorageCashierAccountBalanceGetRequest, BlobStorageCashierAccountBalanceGetResult,
        BlobStorageCashierAccountTopUpRequest, BlobStorageCashierAccountTopUpResult,
    },
    ops::ic::call::CallOps,
    protocol::{
        BLOB_STORAGE_CASHIER_ACCOUNT_BALANCE_GET_V1, BLOB_STORAGE_CASHIER_ACCOUNT_TOP_UP_V1,
        BLOB_STORAGE_CASHIER_STORAGE_GATEWAY_PRINCIPAL_LIST_V1,
    },
};

///
/// CashierClientOps
///
/// Zero-cost namespace for source-backed Cashier inter-canister calls.
///

pub struct CashierClientOps;

impl CashierClientOps {
    pub async fn account_balance_get(
        cashier_canister_id: Principal,
        account: Principal,
    ) -> Result<BlobStorageCashierAccountBalanceGetResult, InternalError> {
        let request = BlobStorageCashierAccountBalanceGetRequest { account };

        CallOps::bounded_wait(
            cashier_canister_id,
            BLOB_STORAGE_CASHIER_ACCOUNT_BALANCE_GET_V1,
        )
        .with_arg(request)?
        .execute()
        .await?
        .candid()
    }

    pub async fn account_top_up(
        cashier_canister_id: Principal,
        request: Option<BlobStorageCashierAccountTopUpRequest>,
        cycles: u128,
    ) -> Result<BlobStorageCashierAccountTopUpResult, InternalError> {
        CallOps::bounded_wait(cashier_canister_id, BLOB_STORAGE_CASHIER_ACCOUNT_TOP_UP_V1)
            .with_arg(request)?
            .with_cycles(cycles)
            .execute()
            .await?
            .candid()
    }

    pub async fn storage_gateway_principal_list(
        cashier_canister_id: Principal,
    ) -> Result<Vec<Principal>, InternalError> {
        CallOps::bounded_wait(
            cashier_canister_id,
            BLOB_STORAGE_CASHIER_STORAGE_GATEWAY_PRINCIPAL_LIST_V1,
        )
        .execute()
        .await?
        .candid()
    }
}
