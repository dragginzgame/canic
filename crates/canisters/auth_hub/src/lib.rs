//!
//! Auth hub canister that provisions auth shard workers for delegated signing.
//!

#![allow(clippy::unused_async)]

use canic::{
    Error,
    api::{access::DelegatedTokenApi, canister::placement::ShardingApi, env::EnvQuery, ic::Call},
    cdk::{types::Principal, utils::time::now_secs},
    dto::auth::{DelegationCert, DelegationProof},
    prelude::*,
    protocol,
};
use canic_internal::canister::AUTH_HUB;

const POOL_NAME: &str = "auth_shards";
const CERT_VERSION: u16 = 1;
const AUTH_SHARD_SET_PROOF: &str = "auth_shard_set_proof";

//
// CANIC
//

canic::start!(AUTH_HUB);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

//
// ENDPOINTS
//

/// create_auth_shard
/// Provision or re-use an auth shard for the provided tenant label.
#[canic_update]
async fn create_auth_shard(tenant: Principal) -> Result<Principal, Error> {
    ShardingApi::assign_to_pool(POOL_NAME, tenant.to_string()).await
}

/// plan_create_auth_shard
/// Dry-run shard allocation decision for a tenant.
#[canic_query]
async fn plan_create_auth_shard(tenant: Principal) -> Result<String, Error> {
    let plan = ShardingApi::plan_assign_to_pool(POOL_NAME, tenant.to_string())?;
    Ok(format!("{plan:?}"))
}

/// provision_auth_shard
/// Prepare a root-signed delegation cert and return it for query retrieval.
#[canic_update]
async fn provision_auth_shard(
    tenant: Principal,
    audiences: Vec<String>,
    scopes: Vec<String>,
    ttl_secs: u64,
) -> Result<(Principal, DelegationCert), Error> {
    if ttl_secs == 0 {
        return Err(Error::invalid("ttl_secs must be greater than zero"));
    }

    let shard_pid = ShardingApi::assign_to_pool(POOL_NAME, tenant.to_string()).await?;

    let root_pid = EnvQuery::view()
        .root_pid
        .ok_or_else(|| Error::internal("root pid unavailable"))?;

    let issued_at = now_secs();
    let expires_at = issued_at.saturating_add(ttl_secs);

    let cert = DelegationCert {
        v: CERT_VERSION,
        signer_pid: shard_pid,
        audiences,
        scopes,
        issued_at,
        expires_at,
    };

    prepare_delegation(root_pid, cert.clone()).await?;

    Ok((shard_pid, cert))
}

/// finalize_auth_shard
/// Install a root-signed delegation proof on the shard.
#[canic_update]
async fn finalize_auth_shard(shard_pid: Principal, proof: DelegationProof) -> Result<(), Error> {
    if proof.cert.signer_pid != shard_pid {
        return Err(Error::invalid("proof signer does not match shard"));
    }

    let root_pid = EnvQuery::view()
        .root_pid
        .ok_or_else(|| Error::internal("root pid unavailable"))?;

    DelegatedTokenApi::verify_delegation_proof(&proof, root_pid)?;
    install_proof(shard_pid, proof).await
}

async fn prepare_delegation(root_pid: Principal, cert: DelegationCert) -> Result<(), Error> {
    let response: Result<(), Error> =
        Call::unbounded_wait(root_pid, protocol::CANIC_DELEGATION_PREPARE)
            .with_arg(cert)
            .execute()
            .await?
            .candid()?;

    response
}

async fn install_proof(shard_pid: Principal, proof: DelegationProof) -> Result<(), Error> {
    let response: Result<(), Error> = Call::unbounded_wait(shard_pid, AUTH_SHARD_SET_PROOF)
        .with_arg(proof)
        .execute()
        .await?
        .candid()?;

    response
}

export_candid!();
