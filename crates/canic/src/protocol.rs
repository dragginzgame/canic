/// Public wire-level endpoint names for Canic canisters.
///
/// `canic_core::protocol` owns names used by runtime inter-canister calls. This
/// facade adds the maintained convenience names used by tooling and consumers.
pub use canic_core::protocol::{
    BLOB_STORAGE_069_GATEWAY_METHODS, BLOB_STORAGE_070_CASHIER_METHODS,
    BLOB_STORAGE_070_GATEWAY_METHODS, BLOB_STORAGE_BLOBS_ARE_LIVE, BLOB_STORAGE_BLOBS_TO_DELETE,
    BLOB_STORAGE_CASHIER_ACCOUNT_BALANCE_GET_V1, BLOB_STORAGE_CASHIER_ACCOUNT_TOP_UP_V1,
    BLOB_STORAGE_CASHIER_STORAGE_GATEWAY_PRINCIPAL_LIST_V1, BLOB_STORAGE_CONFIRM_BLOB_DELETION,
    BLOB_STORAGE_CREATE_CERTIFICATE, BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES, BLOB_STORAGE_STATUS,
    BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS, CANIC_COMMAND, CANIC_STATUS,
};

#[cfg(any(
    feature = "control-plane",
    feature = "wasm-store-canister",
    not(target_arch = "wasm32"),
    test
))]
pub const CANIC_WASM_STORE_CHUNK: &str = "canic_wasm_store_chunk";

#[cfg(any(
    feature = "control-plane",
    feature = "wasm-store-canister",
    not(target_arch = "wasm32"),
    test
))]
pub const CANIC_WASM_STORE_PUBLISH_CHUNK: &str = "canic_wasm_store_publish_chunk";

pub const ICRC10_SUPPORTED_STANDARDS: &str = "icrc10_supported_standards";
pub const ICRC21_CANISTER_CALL_CONSENT_MESSAGE: &str = "icrc21_canister_call_consent_message";
