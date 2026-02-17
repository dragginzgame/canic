///
/// Wire-level endpoint names used across Canic canisters.
/// Keep these synchronized with the macro-defined endpoints.
///

// Root-only endpoints.
pub const CANIC_APP: &str = "canic_app";
pub const CANIC_CANISTER_UPGRADE: &str = "canic_canister_upgrade";
pub const CANIC_RESPONSE: &str = "canic_response";
pub const CANIC_RESPONSE_AUTHENTICATED: &str = "canic_response_authenticated";
pub const CANIC_CANISTER_STATUS: &str = "canic_canister_status";
pub const CANIC_CONFIG: &str = "canic_config";
pub const CANIC_APP_REGISTRY: &str = "canic_app_registry";
pub const CANIC_SUBNET_REGISTRY: &str = "canic_subnet_registry";
pub const CANIC_POOL_LIST: &str = "canic_pool_list";
pub const CANIC_POOL_ADMIN: &str = "canic_pool_admin";
pub const CANIC_DELEGATION_PROVISION: &str = "canic_delegation_provision";
pub const CANIC_REQUEST_DELEGATION: &str = "canic_request_delegation";
pub const CANIC_DELEGATION_SET_SIGNER_PROOF: &str = "canic_delegation_set_signer_proof";
pub const CANIC_DELEGATION_SET_VERIFIER_PROOF: &str = "canic_delegation_set_verifier_proof";

// Non-root sync endpoints.
pub const CANIC_SYNC_STATE: &str = "canic_sync_state";
pub const CANIC_SYNC_TOPOLOGY: &str = "canic_sync_topology";

// IC API endpoints.
pub const IC_CYCLES_ACCEPT: &str = "ic_cycles_accept";

// ICRC endpoints.
pub const ICRC10_SUPPORTED_STANDARDS: &str = "icrc10_supported_standards";
pub const ICRC21_CANISTER_CALL_CONSENT_MESSAGE: &str = "icrc21_canister_call_consent_message";

// Shared endpoints.
pub const CANIC_CANISTER_CYCLE_BALANCE: &str = "canic_canister_cycle_balance";
pub const CANIC_CANISTER_VERSION: &str = "canic_canister_version";
pub const CANIC_TIME: &str = "canic_time";
pub const CANIC_MEMORY_REGISTRY: &str = "canic_memory_registry";
pub const CANIC_ENV: &str = "canic_env";
pub const CANIC_LOG: &str = "canic_log";
pub const CANIC_METRICS_SYSTEM: &str = "canic_metrics_system";
pub const CANIC_METRICS_ICC: &str = "canic_metrics_icc";
pub const CANIC_METRICS_HTTP: &str = "canic_metrics_http";
pub const CANIC_METRICS_TIMER: &str = "canic_metrics_timer";
pub const CANIC_METRICS_ACCESS: &str = "canic_metrics_access";
pub const CANIC_METRICS_DELEGATION: &str = "canic_metrics_delegation";
pub const CANIC_METRICS_PERF: &str = "canic_metrics_perf";
pub const CANIC_METRICS_ENDPOINT_HEALTH: &str = "canic_metrics_endpoint_health";
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

// ICTS endpoints.
pub const ICTS_NAME: &str = "icts_name";
pub const ICTS_VERSION: &str = "icts_version";
pub const ICTS_DESCRIPTION: &str = "icts_description";
pub const ICTS_METADATA: &str = "icts_metadata";
pub const ICTS_CANISTER_STATUS: &str = "icts_canister_status";
