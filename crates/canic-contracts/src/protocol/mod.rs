pub mod application_checkpoint;
pub mod component_directory;
pub mod cycle_balance;
pub mod cycle_history;
pub mod cycle_topups;
pub mod limits;
pub mod profile;
pub mod protected_health;
pub mod public_children;
pub mod public_health;
pub mod public_history;
pub mod public_metrics;
pub mod public_overview;
pub mod wasm_store_chunk;
pub mod wire_declaration;

/// Canonical wire-level endpoint names used by endpoints and inter-canister callers.
///
/// Endpoint emitters and ordinary callers consume these exact names.

pub const CANIC_ROOT_FIXTURE_STATUS: &str = "canic_root_fixture_status";
pub const CANIC_ROOT_MEMBERSHIP: &str = "canic_root_membership";
/// Bounded direct-child inventory.
pub const CANIC_PUBLIC_CHILDREN: &str = "canic_public_children";
/// Immutable role identity and current bootstrap readiness.
pub const CANIC_PUBLIC_OVERVIEW: &str = "canic_public_overview";
/// Bounded cached public history query.
pub const CANIC_PUBLIC_HISTORY: &str = "canic_public_history";

/// Cached metrics endpoint.
pub const CANIC_PUBLIC_METRICS: &str = "canic_public_metrics";

pub const CANIC_PUBLIC_HEALTH: &str = "canic_public_health";
pub const CANIC_ADMISSION_STATUS: &str = "canic_admission_status";
pub const CANIC_WASM_STORE_FIXTURE_CHUNK: &str = "canic_wasm_store_fixture_chunk";
pub const CANIC_WASM_STORE_PUBLISH_FIXTURE: &str = "canic_wasm_store_publish_fixture";

/// Template payload read endpoint owned by Wasm Store.
pub const CANIC_WASM_STORE_CHUNK: &str = "canic_wasm_store_chunk";
/// Template payload publication endpoint owned by Wasm Store.
pub const CANIC_WASM_STORE_PUBLISH_CHUNK: &str = "canic_wasm_store_publish_chunk";
/// Upstream supported standards query.
pub const ICRC10_SUPPORTED_STANDARDS: &str = "icrc10_supported_standards";
/// Upstream consent-message query.
pub const ICRC21_CANISTER_CALL_CONSENT_MESSAGE: &str = "icrc21_canister_call_consent_message";

/// Maximum encoded payload accepted by state and topology cascade endpoints.
pub const CASCADE_SNAPSHOT_MAX_BYTES: usize = 16_384;

pub mod protected_logs;

pub mod protected_metrics;

pub mod protected_memory_allocations;

pub mod child_funding;

pub mod protected_readiness;

pub mod protected_runtime;

pub mod protected_binding;

pub mod caller_authority_status;

pub mod root_config;

pub mod caller_authority;

pub mod inspection_reserve;

pub mod coordinator_acknowledgements;

pub mod observability_relay;
pub mod root_authority;
pub mod root_observation;

pub mod protected_state;

pub mod registry_discovery;
pub mod registry_read;

pub mod release_evidence;

pub mod admission_control;

pub mod pool_retry;

pub mod fleet_state_control;

pub mod cascade;

pub mod runtime_configuration;

pub mod runtime_reporting;

pub mod issuer_proof;

pub mod cycles_capability;

pub mod root_capability;

pub mod issuer_renewal;

pub mod role_attestation;

pub mod issuer_control;

pub mod component_initialization;

pub mod store_inventory;

pub mod store_publication;

pub mod store_gc;

pub mod store_activation;

pub mod store_control;

pub mod store_catalog;

pub mod fixture_bytes;

pub mod store_status;

pub mod template_bytes;

pub mod fixture_reply;

pub mod root_store_reporting;

pub mod provisioning_census;

pub mod pool_census;
pub mod pool_inventory;

pub mod pool_import_read;

pub mod root_activation;

pub mod funding_control;

pub mod cycle_refill;

pub mod component_control;
pub mod component_read;

pub mod root_deletion;

pub mod pool_control;

pub mod registry_control;

pub mod root_store_control;

pub mod retirement_control;

pub mod batch_control;

pub mod inspection_history;

pub mod import_control;

pub mod inspection;

pub mod funding_read;

pub mod subtree_read;

pub mod component_removal_read;

pub mod root_removal_read;

pub mod authority;

pub mod initialization;
