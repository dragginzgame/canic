//! Minimal root stub for delegation access tests.

#![allow(clippy::unused_async)]

use canic::{
    api::canister::{CanisterRole, wasm::WasmApi},
    prelude::*,
};

canic::start_root!();

// Populate the in-memory WASM registry during eager initialization so root
// bootstrap can proceed under minimal test configs.
canic::eager_init!({
    WasmApi::import_static(WASMS);
});

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

// WASM registry entry to satisfy bootstrap invariants and allow
// auto-create of a non-root canister for delegation tests.
const SIGNER_ROLE: CanisterRole = CanisterRole::new("signer");
const SIGNER_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/delegation_signer_stub.wasm"));
const WASMS: &[(CanisterRole, &[u8])] = &[(SIGNER_ROLE, SIGNER_WASM)];

export_candid!();
