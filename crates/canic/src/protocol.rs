/// Public wire-level endpoint names for Canic canisters.
///
/// `canic_contracts::protocol` owns names used by runtime inter-canister calls. This
/// facade adds the maintained convenience names used by tooling and consumers.
pub use canic_contracts::{
    protocol::CANIC_COMMAND, protocol::CANIC_CONTROL_STATUS, protocol::CANIC_COORDINATOR_COMMAND,
    protocol::CANIC_OBSERVABILITY, protocol::CANIC_ROOT_COMMAND,
    protocol::CANIC_ROOT_FIXTURE_STATUS, protocol::CANIC_ROOT_MEMBERSHIP,
    protocol::CANIC_ROOT_STATUS, protocol::CANIC_WASM_STORE_COMMAND,
    protocol::CANIC_WASM_STORE_FIXTURE_CHUNK, protocol::CANIC_WASM_STORE_PUBLISH_FIXTURE,
    protocol::CANIC_WASM_STORE_STATUS, protocol::command_endpoint_for_role,
};

#[cfg(any(
    feature = "control-plane",
    feature = "wasm-store-canister",
    not(target_arch = "wasm32"),
    test
))]
pub use canic_contracts::protocol::CANIC_WASM_STORE_CHUNK;

#[cfg(any(
    feature = "control-plane",
    feature = "wasm-store-canister",
    not(target_arch = "wasm32"),
    test
))]
pub use canic_contracts::protocol::CANIC_WASM_STORE_PUBLISH_CHUNK;

pub use canic_contracts::protocol::ICRC10_SUPPORTED_STANDARDS;
pub use canic_contracts::protocol::ICRC21_CANISTER_CALL_CONSENT_MESSAGE;

pub use crate::__internal::contracts::protocol::{
    CANIC_ADMISSION_STATUS, CANIC_AUTH_STATUS, CANIC_PUBLIC_STATUS, CANIC_ROOT_AUTH_STATUS,
    CANIC_ROOT_OPERATION_STATUS,
};

pub use crate::__internal::contracts::protocol::{
    CANIC_COORDINATOR_OPERATION_STATUS, CANIC_COORDINATOR_REGISTRY, CANIC_WASM_STORE_CATALOG,
};
