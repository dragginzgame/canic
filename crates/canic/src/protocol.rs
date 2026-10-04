/// Public wire-level endpoint names for Canic canisters.
///
/// `canic_core::protocol` owns names used by runtime inter-canister calls. This
/// facade adds the maintained convenience names used by tooling and consumers.
pub use canic_core::protocol::{
    CANIC_COMMAND, CANIC_CONTROL_STATUS, CANIC_COORDINATOR_COMMAND, CANIC_OBSERVABILITY,
    CANIC_ROOT_COMMAND, CANIC_ROOT_FIXTURE_STATUS, CANIC_ROOT_STATUS, CANIC_WASM_STORE_COMMAND,
    CANIC_WASM_STORE_FIXTURE_CHUNK, CANIC_WASM_STORE_PUBLISH_FIXTURE, CANIC_WASM_STORE_STATUS,
    command_endpoint_for_role,
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

pub use crate::__internal::core::protocol::{
    CANIC_ADMISSION_STATUS, CANIC_AUTH_STATUS, CANIC_PUBLIC_STATUS, CANIC_ROOT_AUTH_STATUS,
    CANIC_ROOT_OPERATION_STATUS,
};

pub use crate::__internal::core::protocol::{
    CANIC_COORDINATOR_OPERATION_STATUS, CANIC_COORDINATOR_REGISTRY, CANIC_WASM_STORE_CATALOG,
};
