#![allow(clippy::unused_async)]

use canic::{core::ops::signature, prelude::*};
use canic_internal::canister::AUTH;

//
// CANIC
//

canic::start!(AUTH);

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

//
// ENDPOINTS
//

#[canic_update]
fn authenticate_caller() -> Vec<u8> {
    // step 1: prepare the signature
    signature::prepare(b"domain", b"user-auth", b"hello");

    // returning root_hash is optional — just for debugging
    signature::root_hash()
}

#[canic_query]
fn get_auth_signature() -> Option<Vec<u8>> {
    signature::get(b"domain", b"user-auth", b"hello")
}

export_candid!();
