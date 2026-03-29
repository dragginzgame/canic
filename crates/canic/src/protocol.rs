/// Public wire-level endpoint names for Canic canisters.
///
/// `canic_core::protocol` is intentionally the smaller runtime-internal subset
/// used by orchestration code. This facade module keeps the wider convenience
/// table for tests, tooling, and downstream callers.
pub use canic_core::protocol::{
    CANIC_ATTESTATION_KEY_SET, CANIC_DELEGATION_SET_SIGNER_PROOF,
    CANIC_DELEGATION_SET_VERIFIER_PROOF, CANIC_REQUEST_DELEGATION, CANIC_REQUEST_ROLE_ATTESTATION,
    CANIC_RESPONSE_CAPABILITY_V1, CANIC_SYNC_STATE, CANIC_SYNC_TOPOLOGY, CANIC_WASM_STORE_BEGIN_GC,
    CANIC_WASM_STORE_BOOTSTRAP_PREPARE_ADMIN, CANIC_WASM_STORE_BOOTSTRAP_PUBLISH_CHUNK_ADMIN,
    CANIC_WASM_STORE_BOOTSTRAP_RESUME_ROOT_ADMIN, CANIC_WASM_STORE_BOOTSTRAP_STAGE_MANIFEST_ADMIN,
    CANIC_WASM_STORE_CATALOG, CANIC_WASM_STORE_CHUNK, CANIC_WASM_STORE_COMPLETE_GC,
    CANIC_WASM_STORE_INFO, CANIC_WASM_STORE_OVERVIEW, CANIC_WASM_STORE_PREPARE,
    CANIC_WASM_STORE_PREPARE_GC, CANIC_WASM_STORE_PUBLISH_CHUNK, CANIC_WASM_STORE_STATUS,
};

pub const CANIC_APP: &str = "canic_app";
pub const CANIC_CANISTER_UPGRADE: &str = "canic_canister_upgrade";
pub const CANIC_CANISTER_STATUS: &str = "canic_canister_status";
pub const CANIC_CONFIG: &str = "canic_config";
pub const CANIC_APP_REGISTRY: &str = "canic_app_registry";
pub const CANIC_SUBNET_REGISTRY: &str = "canic_subnet_registry";
pub const CANIC_POOL_LIST: &str = "canic_pool_list";
pub const CANIC_POOL_ADMIN: &str = "canic_pool_admin";
pub const CANIC_DELEGATION_ADMIN: &str = "canic_delegation_admin";
pub const CANIC_IC_CYCLES_ACCEPT: &str = "canic_ic_cycles_accept";
pub const ICRC10_SUPPORTED_STANDARDS: &str = "icrc10_supported_standards";
pub const ICRC21_CANISTER_CALL_CONSENT_MESSAGE: &str = "icrc21_canister_call_consent_message";
pub const CANIC_CANISTER_CYCLE_BALANCE: &str = "canic_canister_cycle_balance";
pub const CANIC_CANISTER_VERSION: &str = "canic_canister_version";
pub const CANIC_TIME: &str = "canic_time";
pub const CANIC_MEMORY_REGISTRY: &str = "canic_memory_registry";
pub const CANIC_ENV: &str = "canic_env";
pub const CANIC_LOG: &str = "canic_log";
pub const CANIC_METRICS: &str = "canic_metrics";
pub const CANIC_READY: &str = "canic_ready";
pub const CANIC_APP_STATE: &str = "canic_app_state";
pub const CANIC_SUBNET_STATE: &str = "canic_subnet_state";
pub const CANIC_APP_DIRECTORY: &str = "canic_app_directory";
pub const CANIC_SUBNET_DIRECTORY: &str = "canic_subnet_directory";
pub const CANIC_CANISTER_CHILDREN: &str = "canic_canister_children";
pub const CANIC_CYCLE_TRACKER: &str = "canic_cycle_tracker";
pub const CANIC_SCALING_REGISTRY: &str = "canic_scaling_registry";
pub const CANIC_SHARDING_REGISTRY: &str = "canic_sharding_registry";
pub const CANIC_SHARDING_PARTITION_KEYS: &str = "canic_sharding_partition_keys";
pub const CANIC_STANDARDS: &str = "canic_standards";
