//! Consumer-owned canister shell; App configuration belongs to this package.

canic_blob_service::canister!();

#[cfg(all(test, feature = "wire-schema", not(target_arch = "wasm32")))]
mod schema;
