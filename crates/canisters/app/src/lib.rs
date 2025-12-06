#![allow(clippy::unused_async)]

use candid::Principal;
use canic::{
    core::{Error, ops::signature},
    prelude::*,
    utils::time::now_secs,
};
use canic_internal::{AuthToken, canister::APP};

//
// CANIC
//

canic::start!(APP);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

///
/// Example protected update call that requires a valid token.
///
#[update]
async fn verify(
    domain: Vec<u8>,
    seed: Vec<u8>,
    message: Vec<u8>,
    signature_cbor: Vec<u8>,
    issuer_pid: Principal,
) -> Result<String, Error> {
    signature::verify(&domain, &seed, &message, &signature_cbor, issuer_pid)?;

    // 3️⃣ Parse the AuthToken from CBOR
    let token: AuthToken = signature::parse_message(&message)?;
    let expiry = token.exp;

    // 4️⃣ Expiry check
    let now = now_secs();
    if expiry <= now {
        return Err(Error::custom("token expired"));
    }

    // from here, `user` is the verified authenticated user
    Ok(format!(
        "authenticated! issuer: {issuer_pid} expiry: {expiry}"
    ))
}

// end
export_candid!();
