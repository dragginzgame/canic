//! Minimal Fleet Subnet Root stub for Registry lifecycle tests.

#![expect(clippy::unused_async)]

use canic::{Error, api::auth::AuthApi, dto::auth::SignedRoleAttestation, prelude::*};

canic::start_fleet_root!();

#[canic_update(public)]
async fn root_verify_role_attestation(
    attestation: SignedRoleAttestation,
    min_accepted_epoch: u64,
) -> Result<(), Error> {
    AuthApi::verify_role_attestation(&attestation, min_accepted_epoch).await
}

#[canic_query(public)]
async fn root_now_secs() -> Result<u64, Error> {
    Ok(ic_cdk::api::time() / 1_000_000_000)
}

#[canic_update(requires(caller::is_controller()))]
async fn test_provision_chain_key_delegation_proof_for_issuer(
    issuer_pid: candid::Principal,
) -> Result<(), Error> {
    AuthApi::provision_chain_key_delegation_proof_for_issuer_root(issuer_pid).await
}

canic::finish!();
