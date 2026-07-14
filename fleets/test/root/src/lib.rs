#![expect(clippy::unused_async)]

use canic::{
    Error,
    api::auth::AuthApi,
    cdk::{
        call::Call,
        candid::{CandidType, Deserialize, Principal},
    },
    prelude::*,
};

canic::start!();

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

#[derive(CandidType)]
struct TestEcdsaPublicKeyArgs {
    canister_id: Option<Principal>,
    derivation_path: Vec<Vec<u8>>,
    key_id: TestEcdsaKeyId,
}

#[derive(CandidType)]
struct TestEcdsaKeyId {
    curve: TestEcdsaCurve,
    name: String,
}

#[derive(CandidType, Deserialize)]
enum TestEcdsaCurve {
    #[serde(rename = "secp256k1")]
    Secp256k1,
}

#[derive(CandidType, Deserialize)]
struct TestEcdsaPublicKeyResult {
    public_key: Vec<u8>,
    chain_code: Vec<u8>,
}

#[canic_update(requires(caller::is_controller()))]
async fn test_chain_key_ecdsa_public_key(
    canister_id: Principal,
    key_name: String,
    derivation_path: Vec<Vec<u8>>,
) -> Result<Vec<u8>, Error> {
    let response = Call::bounded_wait(Principal::management_canister(), "ecdsa_public_key")
        .with_arg(TestEcdsaPublicKeyArgs {
            canister_id: Some(canister_id),
            derivation_path,
            key_id: TestEcdsaKeyId {
                curve: TestEcdsaCurve::Secp256k1,
                name: key_name,
            },
        })
        .await
        .map_err(|err| Error::internal(format!("ecdsa_public_key failed: {err}")))?;
    let response: TestEcdsaPublicKeyResult = response
        .candid()
        .map_err(|err| Error::internal(format!("ecdsa_public_key response failed: {err}")))?;

    Ok(response.public_key)
}

#[canic_update(requires(caller::is_controller()))]
async fn test_provision_chain_key_delegation_proof_for_issuer(
    issuer_pid: Principal,
) -> Result<(), Error> {
    AuthApi::provision_chain_key_delegation_proof_for_issuer_root(issuer_pid).await
}

canic::finish!();
