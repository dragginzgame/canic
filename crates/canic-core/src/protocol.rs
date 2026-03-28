/// Runtime wire-level endpoint names used by `canic-core` for inter-canister calls.
///
/// Keep these synchronized with the macro-defined endpoints.

pub const CANIC_RESPONSE_CAPABILITY_V1: &str = "canic_response_capability_v1";
pub const CANIC_REQUEST_DELEGATION: &str = "canic_request_delegation";
pub const CANIC_REQUEST_ROLE_ATTESTATION: &str = "canic_request_role_attestation";
pub const CANIC_ATTESTATION_KEY_SET: &str = "canic_attestation_key_set";
pub const CANIC_DELEGATION_SET_SIGNER_PROOF: &str = "canic_delegation_set_signer_proof";
pub const CANIC_DELEGATION_SET_VERIFIER_PROOF: &str = "canic_delegation_set_verifier_proof";
pub const CANIC_WASM_STORE_CATALOG: &str = "canic_wasm_store_catalog";
pub const CANIC_WASM_STORE_INFO: &str = "canic_wasm_store_info";
pub const CANIC_WASM_STORE_STATUS: &str = "canic_wasm_store_status";
pub const CANIC_WASM_STORE_PREPARE_GC: &str = "canic_wasm_store_prepare_gc";
pub const CANIC_WASM_STORE_BEGIN_GC: &str = "canic_wasm_store_begin_gc";
pub const CANIC_WASM_STORE_COMPLETE_GC: &str = "canic_wasm_store_complete_gc";
pub const CANIC_WASM_STORE_PREPARE: &str = "canic_wasm_store_prepare";
pub const CANIC_WASM_STORE_CHUNK: &str = "canic_wasm_store_chunk";
pub const CANIC_WASM_STORE_PUBLISH_CHUNK: &str = "canic_wasm_store_publish_chunk";
pub const CANIC_WASM_STORE_BOOTSTRAP_STAGE_MANIFEST_ADMIN: &str =
    "canic_wasm_store_bootstrap_stage_manifest_admin";
pub const CANIC_WASM_STORE_BOOTSTRAP_PREPARE_ADMIN: &str =
    "canic_wasm_store_bootstrap_prepare_admin";
pub const CANIC_WASM_STORE_BOOTSTRAP_PUBLISH_CHUNK_ADMIN: &str =
    "canic_wasm_store_bootstrap_publish_chunk_admin";
pub const CANIC_WASM_STORE_BOOTSTRAP_RESUME_ROOT_ADMIN: &str =
    "canic_wasm_store_bootstrap_resume_root_admin";

pub const CANIC_SYNC_STATE: &str = "canic_sync_state";
pub const CANIC_SYNC_TOPOLOGY: &str = "canic_sync_topology";
