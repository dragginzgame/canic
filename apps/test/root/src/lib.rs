#![expect(clippy::unused_async)]

use candid::{CandidType, Deserialize, Principal};
use canic::{Error, api::auth::AuthApi, prelude::*};
use ic_cdk::call::Call;

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
struct TestCanisterIdArgs {
    canister_id: Principal,
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
        .map_err(|err| Error::from_registered(canic::diagnostics::codes::STATE_FAILED))?;
    let response: TestEcdsaPublicKeyResult = response
        .candid()
        .map_err(|err| Error::from_registered(canic::diagnostics::codes::STATE_FAILED))?;

    Ok(response.public_key)
}

#[canic_update(requires(caller::is_controller()))]
async fn test_provision_chain_key_delegation_proof_for_issuer(
    issuer_pid: Principal,
) -> Result<(), Error> {
    AuthApi::provision_chain_key_delegation_proof_for_issuer_root(issuer_pid).await
}

#[canic_update(internal, requires(caller::is_controller()))]
async fn test_set_canister_running(canister_id: Principal, running: bool) -> Result<(), Error> {
    let method = if running {
        "start_canister"
    } else {
        "stop_canister"
    };
    Call::unbounded_wait(Principal::management_canister(), method)
        .with_arg(TestCanisterIdArgs { canister_id })
        .await
        .map_err(|err| Error::from_registered(canic::diagnostics::codes::STATE_FAILED))?;

    Ok(())
}

canic::finish!();
