#![expect(clippy::unused_async)]

use canic::{
    Error,
    api::auth::AuthApi,
    dto::auth::{DelegatedToken, DelegatedTokenMintRequest},
    ids::cap,
    prelude::*,
};

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

/// Issue a self-contained delegated token without local proof-cache state.
#[canic_update]
async fn user_shard_issue_token(
    request: DelegatedTokenMintRequest,
) -> Result<DelegatedToken, Error> {
    canic::access::require_local()?;
    AuthApi::mint_token(request).await
}

#[cfg(not(canic_disable_bundle_observability_env))]
#[canic_update]
async fn user_shard_local_public_key_test() -> Result<Vec<u8>, Error> {
    canic::access::require_local()?;
    AuthApi::local_shard_public_key_sec1().await
}

#[canic_query(requires(auth::authenticated(cap::VERIFY)))]
async fn hello(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;

    Ok(())
}

canic::finish!();
