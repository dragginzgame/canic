//!
//! Auth shard canister that stores delegation proofs and mints delegated tokens.
//!

#![allow(clippy::unused_async)]

use canic::{
    Error,
    api::{access::DelegatedTokenApi, auth::DelegationApi, env::EnvQuery},
    cdk::api::canister_self,
    dto::auth::{DelegatedToken, DelegatedTokenClaims, DelegationProof},
    prelude::*,
};
use canic_internal::canister::AUTH_SHARD;

const TOKEN_VERSION: u16 = 1;

//
// CANIC
//

canic::start!(AUTH_SHARD);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

//
// ENDPOINTS
//

/// auth_shard_set_proof
/// Store a root-signed delegation proof for this shard.
#[canic_update(auth(caller_is_registered_to_subnet))]
async fn auth_shard_set_proof(proof: DelegationProof) -> Result<(), Error> {
    let self_pid = canister_self();
    if proof.cert.signer_pid != self_pid {
        return Err(Error::forbidden(
            "delegation signer does not match canister",
        ));
    }

    DelegatedTokenApi::verify_delegation_structure(&proof, Some(self_pid))?;

    let root_pid = EnvQuery::view()
        .root_pid
        .ok_or_else(|| Error::internal("root pid unavailable"))?;

    DelegatedTokenApi::verify_delegation_signature(&proof, root_pid)?;
    DelegationApi::store_proof(proof)
}

/// auth_shard_mint_token
/// Mint a delegated token using the locally stored delegation proof.
#[canic_update]
async fn auth_shard_mint_token(claims: DelegatedTokenClaims) -> Result<DelegatedToken, Error> {
    let proof = DelegationApi::require_proof()?;
    DelegatedTokenApi::sign_token(TOKEN_VERSION, claims, proof)
}

export_candid!();
