#![expect(clippy::unused_async)]

use canic::{
    Error,
    cdk::{
        mgmt::{EcdsaCurve, EcdsaKeyId, EcdsaPublicKeyArgs, ecdsa_public_key},
        types::Principal,
    },
    prelude::*,
};

canic::start!();

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

#[canic_update(requires(caller::is_controller()))]
async fn test_chain_key_ecdsa_public_key(
    canister_id: Principal,
    key_name: String,
    derivation_path: Vec<Vec<u8>>,
) -> Result<Vec<u8>, Error> {
    let response = ecdsa_public_key(&EcdsaPublicKeyArgs {
        canister_id: Some(canister_id),
        derivation_path,
        key_id: EcdsaKeyId {
            curve: EcdsaCurve::Secp256k1,
            name: key_name,
        },
    })
    .await
    .map_err(|err| Error::internal(format!("ecdsa_public_key failed: {err}")))?;

    Ok(response.public_key)
}

canic::finish!();
