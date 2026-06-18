#![expect(clippy::unused_async)]

use canic::{Error, dto::auth::DelegatedToken, ids::cap, prelude::*};

#[cfg(canic_test_delegation_material)]
use canic::{
    cdk::{call::Call, candid::encode_args, types::Principal},
    dto::auth::{DelegationProof, DelegationProofGetRequest},
    protocol,
};

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[canic_query(requires(auth::authenticated(cap::VERIFY)))]
async fn hello(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;

    Ok(())
}

#[cfg(canic_test_delegation_material)]
#[canic_query(composite)]
async fn user_shard_test_nested_root_get_delegation_proof(
    root_pid: Principal,
    request: DelegationProofGetRequest,
) -> Result<DelegationProof, Error> {
    let args = encode_args((request,)).map_err(|err| {
        Error::internal(format!("encode nested root proof get args failed: {err}"))
    })?;
    let response = Call::bounded_wait(root_pid, protocol::CANIC_GET_DELEGATION_PROOF)
        .with_raw_args(&args)
        .await
        .map_err(|err| Error::internal(format!("nested root proof get call failed: {err:?}")))?;

    response
        .candid::<Result<DelegationProof, Error>>()
        .map_err(|err| {
            Error::internal(format!(
                "decode nested root proof get response failed: {err}"
            ))
        })?
}

canic::finish!();
