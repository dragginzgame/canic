#![expect(clippy::unused_async)]

use candid::Principal;
use canic::{
    Error,
    dto::blob_storage::{
        BlobStorageBillingConfig, BlobStorageCashierAccountTopUpRequest,
        BlobStorageCashierAccountTopUpResult, BlobStorageLocalCounters,
    },
    prelude::*,
};
use ic_cdk::api::time;

canic::start!();

/// Run no-op setup for the blob-storage probe.
async fn canic_setup() {}

/// Accept no install payload for the blob-storage probe.
async fn canic_install(_: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for the blob-storage probe.
async fn canic_upgrade() {}

canic::canic_emit_blob_storage_endpoints!(guard = caller::is_controller());
canic::canic_emit_blob_storage_billing_endpoints!(
    sync_gateway_principals_guard = caller::is_controller(),
    fund_from_cycles_guard = caller::is_controller(),
    status_guard = caller::is_controller(),
);

/// Register one authorized immutable object storage gateway principal.
#[canic_update(requires(caller::is_controller()))]
async fn blob_storage_probe_add_gateway(principal: Principal) -> Result<(), Error> {
    canic::api::blob_storage::BlobStorageApi::upsert_gateway_principal(principal, time());
    Ok(())
}

/// Remove one authorized immutable object storage gateway principal.
#[canic_update(requires(caller::is_controller()))]
async fn blob_storage_probe_remove_gateway(principal: Principal) -> Result<bool, Error> {
    Ok(canic::api::blob_storage::BlobStorageApi::remove_gateway_principal(principal))
}

/// Configure blob-storage billing for local tests.
#[canic_update(requires(caller::is_controller()))]
async fn blob_storage_probe_configure_billing(
    config: BlobStorageBillingConfig,
) -> Result<(), Error> {
    canic::api::blob_storage::BlobStorageApi::configure_billing(config)
}

/// Sync authorized immutable object storage gateway principals from a mock Cashier.
#[canic_update(requires(caller::is_controller()))]
async fn blob_storage_probe_sync_gateways_from_cashier(
    cashier_canister_id: Principal,
    max_gateway_principals: u64,
) -> Result<u64, Error> {
    let max_gateway_principals = usize::try_from(max_gateway_principals)
        .map_err(|_| Error::invalid("max_gateway_principals exceeds usize"))?;

    canic::api::blob_storage::BlobStorageApi::sync_gateway_principals_from_cashier(
        cashier_canister_id,
        max_gateway_principals,
    )
    .await
}

/// Sync authorized immutable object storage gateway principals from configured Cashier.
#[canic_update(requires(caller::is_controller()))]
async fn blob_storage_probe_sync_gateways_from_configured_cashier() -> Result<u64, Error> {
    canic::api::blob_storage::BlobStorageApi::sync_gateway_principals_from_configured_cashier()
        .await
}

/// Read the total Cashier balance for one account through the billing API wrapper.
#[canic_update(requires(caller::is_controller()))]
async fn blob_storage_probe_cashier_total_balance(
    cashier_canister_id: Principal,
    account: Principal,
) -> Result<u128, Error> {
    canic::api::blob_storage::BlobStorageApi::cashier_account_total_balance(
        cashier_canister_id,
        account,
    )
    .await
}

/// Top up a Cashier account from this probe canister's cycles.
#[canic_update(requires(caller::is_controller()))]
async fn blob_storage_probe_cashier_top_up(
    cashier_canister_id: Principal,
    account: Principal,
    cycles: u128,
) -> Result<BlobStorageCashierAccountTopUpResult, Error> {
    canic::api::blob_storage::BlobStorageApi::cashier_account_top_up(
        cashier_canister_id,
        Some(BlobStorageCashierAccountTopUpRequest {
            target_balance: None,
            account: Some(account),
        }),
        cycles,
    )
    .await
}

/// Return stored, pending-deletion, and gateway-principal counts for local tests.
#[canic_query(public)]
fn blob_storage_probe_counts() -> Result<BlobStorageLocalCounters, Error> {
    Ok(canic::api::blob_storage::BlobStorageApi::local_counters())
}

/// Mark a live blob as pending object storage gateway deletion.
#[canic_update(requires(caller::is_controller()))]
async fn blob_storage_probe_mark_pending_delete(root_hash: String) -> Result<bool, Error> {
    canic::api::blob_storage::BlobStorageApi::mark_pending_delete(&root_hash, time())
}

/// Return whether a blob is live and not pending deletion.
#[canic_query(public)]
fn blob_storage_probe_is_live(root_hash: String) -> Result<bool, Error> {
    canic::api::blob_storage::BlobStorageApi::is_live(&root_hash)
}

canic::finish!();
