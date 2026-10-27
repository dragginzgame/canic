//! Minimal Fleet Subnet Root stub for Registry lifecycle tests.

#![expect(clippy::unused_async)]

mod protocol;

use canic::{Error, api::auth::AuthApi, prelude::*};

canic::start_fleet_root!();

#[canic_update(requires(caller::is_controller()), wire(id = protocol::PROVISION, request = protocol::LIMITS, response = protocol::LIMITS))]
async fn test_provision_chain_key_delegation_proof_for_issuer(
    issuer_pid: ic_platform::Principal,
) -> Result<(), Error> {
    AuthApi::provision_chain_key_delegation_proof_for_issuer_root(issuer_pid.into()).await
}

canic::finish!();
