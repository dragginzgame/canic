#![expect(clippy::unused_async)]

use canic::{Error, cdk::types::Principal, prelude::*};

canic::start!();

/// Run no-op setup for the blob-storage probe.
async fn canic_setup() {}

/// Accept no install payload for the blob-storage probe.
async fn canic_install(_: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for the blob-storage probe.
async fn canic_upgrade() {}

canic::canic_emit_blob_storage_endpoints!(guard = caller::is_controller());

/// Register one authorized immutable object storage gateway principal.
#[canic_update(requires(caller::is_controller()))]
async fn blob_storage_probe_add_gateway(principal: Principal) -> Result<(), Error> {
    canic::api::blob_storage::BlobStorageApi::upsert_gateway_principal(
        principal,
        canic::cdk::utils::time::now_nanos(),
    );
    Ok(())
}

/// Remove one authorized immutable object storage gateway principal.
#[canic_update(requires(caller::is_controller()))]
async fn blob_storage_probe_remove_gateway(principal: Principal) -> Result<bool, Error> {
    Ok(canic::api::blob_storage::BlobStorageApi::remove_gateway_principal(principal))
}

/// Return stored, pending-deletion, and gateway-principal counts for local tests.
#[canic_query]
fn blob_storage_probe_counts() -> Result<(u64, u64, u64), Error> {
    Ok((
        canic::api::blob_storage::BlobStorageApi::stored_blob_count(),
        canic::api::blob_storage::BlobStorageApi::pending_deletion_count(),
        canic::api::blob_storage::BlobStorageApi::gateway_principal_count(),
    ))
}

/// Mark a live blob as pending object storage gateway deletion.
#[canic_update(requires(caller::is_controller()))]
async fn blob_storage_probe_mark_pending_delete(root_hash: String) -> Result<bool, Error> {
    canic::api::blob_storage::BlobStorageApi::mark_pending_delete(
        &root_hash,
        canic::cdk::utils::time::now_nanos(),
    )
}

/// Return whether a blob is live and not pending deletion.
#[canic_query]
fn blob_storage_probe_is_live(root_hash: String) -> Result<bool, Error> {
    canic::api::blob_storage::BlobStorageApi::is_live(&root_hash)
}

canic::finish!();
