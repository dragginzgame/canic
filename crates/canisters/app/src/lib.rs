#![allow(clippy::unused_async)]

use candid::Principal;
use canic::{Error, ops::signature, prelude::*, utils::time::now_secs};
use canic_internal::{AuthToken, canister::APP};

//
// CANIC
//

canic_start!(APP);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

///
/// Example protected update call that requires a valid token.
///
#[update]
async fn verify(
    message: Vec<u8>,
    signature_cbor: Vec<u8>,
    issuer_pid: Principal,
) -> Result<String, Error> {
    signature::verify(&message, &signature_cbor, issuer_pid)?;

    // 3️⃣ Parse the AuthToken from CBOR
    let token: AuthToken = signature::parse_message(&message)?;
    let expiry = token.exp;

    // 4️⃣ Expiry check
    assert!(expiry > now_secs(), "token expired");

    // from here, `user` is the verified authenticated user
    Ok(format!(
        "authenticated! issuer: {issuer_pid} expiry: {expiry}"
    ))
}

// end
export_candid!();
