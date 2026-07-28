/// Runtime wire-level endpoint names used by `canic-core` for inter-canister calls.
///
/// Keep these synchronized with the macro-defined endpoints.

pub const CANIC_RESPONSE_CAPABILITY_V1: &str = "canic_response_capability_v1";
pub const CANIC_FLEET_REGISTRY: &str = "canic_fleet_registry";
pub const CANIC_FLEET_REGISTRY_MANIFEST: &str = "canic_fleet_registry_manifest";
pub const CANIC_FLEET_REGISTRY_VERSION: &str = "canic_fleet_registry_version";
pub const CANIC_FLEET_SUBNET_ROOT_JOIN: &str = "canic_fleet_subnet_root_join";
pub const CANIC_FLEET_REGISTRY_SNAPSHOT_FOR_ROOT: &str = "canic_fleet_registry_snapshot_for_root";
pub const CANIC_FLEET_REGISTRY_ACKNOWLEDGE_ROOT: &str = "canic_fleet_registry_acknowledge_root";
pub const CANIC_FLEET_REGISTRY_ROOT_ACKNOWLEDGEMENTS: &str =
    "canic_fleet_registry_root_acknowledgements";
pub const CANIC_FLEET_REGISTRY_ACTIVATE: &str = "canic_fleet_registry_activate";
pub const CANIC_FLEET_SUBNET_ROOT_AUTHORITY: &str = "canic_fleet_subnet_root_authority";
pub const CANIC_FLEET_SUBNET_ROOT_CANISTER_SUMMARY: &str =
    "canic_fleet_subnet_root_canister_summary";
pub const CANIC_FLEET_REGISTRY_SYNCHRONIZE: &str = "canic_fleet_registry_synchronize";
pub const CANIC_FLEET_REGISTRY_SYNC_STATUS: &str = "canic_fleet_registry_sync_status";
pub const CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR: &str = "canic_fleet_registry_activate_mirror";
pub const CANIC_FLEET_REGISTRY_MIRROR_STATUS: &str = "canic_fleet_registry_mirror_status";
pub const CANIC_ROOT_COMPONENT_REGISTRY_PREPARE: &str = "canic_root_component_registry_prepare";
pub const CANIC_ROOT_COMPONENT_REGISTRY_STATUS: &str = "canic_root_component_registry_status";
pub const CANIC_ROOT_COMPONENT_ALLOCATE: &str = "canic_root_component_allocate";
pub const CANIC_ROOT_COMPONENT_ALLOCATION_STATUS: &str = "canic_root_component_allocation_status";
pub const CANIC_ROOT_COMPONENT_CREATE: &str = "canic_root_component_create";
pub const CANIC_ROOT_COMPONENT_INSTALL: &str = "canic_root_component_install";
pub const CANIC_ROOT_COMPONENT_COMMIT: &str = "canic_root_component_commit";
pub const CANIC_ROOT_COMPONENT_DIRECTORY_PREPARE: &str = "canic_root_component_directory_prepare";
pub const CANIC_ROOT_COMPONENT_RUNTIME_ACTIVATE: &str = "canic_root_component_runtime_activate";
pub const CANIC_ROOT_COMPONENT_MEMBERSHIP_ACTIVATE: &str =
    "canic_root_component_membership_activate";
pub const CANIC_ROOT_COMPONENT_REGISTRY_PARTITION: &str = "canic_root_component_registry_partition";
pub const CANIC_ROOT_COMPONENT_DIRECTORY_HEAD: &str = "canic_root_component_directory_head";
pub const CANIC_COMPONENT_RUNTIME_DIRECTORY_PREPARE: &str =
    "canic_component_runtime_directory_prepare";
pub const CANIC_COMPONENT_RUNTIME_DIRECTORY_SYNCHRONIZE: &str =
    "canic_component_runtime_directory_synchronize";
pub const CANIC_COMPONENT_RUNTIME_STATUS: &str = "canic_component_runtime_status";
pub const CANIC_COMPONENT_RUNTIME_ACTIVATE: &str = "canic_component_runtime_activate";
pub const CANIC_FLEET_ACTIVATION_STATUS: &str = "canic_fleet_activation_status";
pub const CANIC_MANAGED_CANISTER_BINDING: &str = "canic_managed_canister_binding";
pub const CANIC_PREPARE_FLEET_ACTIVATION: &str = "canic_prepare_fleet_activation";
pub const CANIC_RESUME_FLEET_ACTIVATION: &str = "canic_resume_fleet_activation";
pub const CANIC_PREPARE_FLEET_CREDENTIAL_GENERATION: &str =
    "canic_prepare_fleet_credential_generation";
pub const CANIC_ACTIVATE_FLEET: &str = "canic_activate_fleet";
pub const CANIC_UPSERT_ROOT_ISSUER_POLICY: &str = "canic_upsert_root_issuer_policy";
pub const CANIC_UPSERT_ROOT_ISSUER_RENEWAL_TEMPLATE: &str =
    "canic_upsert_root_issuer_renewal_template";
pub const CANIC_ROOT_ISSUER_RENEWAL_STATUS: &str = "canic_root_issuer_renewal_status";
pub const CANIC_GET_OR_CREATE_CHAIN_KEY_DELEGATION_PROOF: &str =
    "canic_get_or_create_chain_key_delegation_proof";
pub const CANIC_PREPARE_DELEGATED_TOKEN: &str = "canic_prepare_delegated_token";
pub const CANIC_GET_DELEGATED_TOKEN: &str = "canic_get_delegated_token";
pub const CANIC_ACTIVE_DELEGATION_PROOF_STATUS: &str = "canic_active_delegation_proof_status";
pub const CANIC_PREPARE_ROLE_ATTESTATION: &str = "canic_prepare_role_attestation";
pub const CANIC_GET_ROLE_ATTESTATION: &str = "canic_get_role_attestation";
pub const CANIC_INSTALL_ACTIVE_DELEGATION_PROOF: &str = "canic_install_active_delegation_proof";
pub const CANIC_BOOTSTRAP_STATUS: &str = "canic_bootstrap_status";
pub const CANIC_HEALTH: &str = "canic_health";
pub const CANIC_READINESS: &str = "canic_readiness";
pub const CANIC_RUNTIME_STATUS: &str = "canic_runtime_status";
pub const CANIC_CYCLE_BALANCE: &str = "canic_cycle_balance";
pub const CANIC_CYCLE_TRACKER: &str = "canic_cycle_tracker";
pub const CANIC_CYCLE_TOPUPS: &str = "canic_cycle_topups";
pub const CANIC_METADATA: &str = "canic_metadata";
pub const CANIC_WASM_STORE_CATALOG: &str = "canic_wasm_store_catalog";
pub const CANIC_WASM_STORE_INFO: &str = "canic_wasm_store_info";
pub const CANIC_WASM_STORE_STATUS: &str = "canic_wasm_store_status";
pub const CANIC_WASM_STORE_PREPARE_GC: &str = "canic_wasm_store_prepare_gc";
pub const CANIC_WASM_STORE_BEGIN_GC: &str = "canic_wasm_store_begin_gc";
pub const CANIC_WASM_STORE_COMPLETE_GC: &str = "canic_wasm_store_complete_gc";
pub const CANIC_WASM_STORE_PREPARE: &str = "canic_wasm_store_prepare";
pub const CANIC_WASM_STORE_CHUNK: &str = "canic_wasm_store_chunk";
pub const CANIC_WASM_STORE_PUBLISH_CHUNK: &str = "canic_wasm_store_publish_chunk";
pub const CANIC_WASM_STORE_STAGE_MANIFEST: &str = "canic_wasm_store_stage_manifest";
pub const CANIC_WASM_STORE_BOOTSTRAP_DEBUG: &str = "canic_wasm_store_bootstrap_debug";
pub const CANIC_ROOT_STORE_BOOTSTRAP: &str = "canic_root_store_bootstrap";
pub const CANIC_ROOT_STORE_BOOTSTRAP_STATUS: &str = "canic_root_store_bootstrap_status";
pub const CANIC_WASM_STORE_OVERVIEW: &str = "canic_wasm_store_overview";
pub const CANIC_TEMPLATE_PREPARE_ADMIN: &str = "canic_template_prepare_admin";
pub const CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN: &str = "canic_template_publish_chunk_admin";
pub const CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN: &str = "canic_template_stage_manifest_admin";
pub const BLOB_STORAGE_BLOBS_ARE_LIVE: &str = "_immutableObjectStorageBlobsAreLive";
pub const BLOB_STORAGE_BLOBS_TO_DELETE: &str = "_immutableObjectStorageBlobsToDelete";
pub const BLOB_STORAGE_CONFIRM_BLOB_DELETION: &str = "_immutableObjectStorageConfirmBlobDeletion";
pub const BLOB_STORAGE_CREATE_CERTIFICATE: &str = "_immutableObjectStorageCreateCertificate";
pub const BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS: &str =
    "_immutableObjectStorageUpdateGatewayPrincipals";
pub const BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES: &str =
    "_immutableObjectStorageFundFromProjectCycles";
pub const BLOB_STORAGE_STATUS: &str = "get_blob_storage_status";
pub const BLOB_STORAGE_CASHIER_ACCOUNT_BALANCE_GET_V1: &str = "account_balance_get_v1";
pub const BLOB_STORAGE_CASHIER_ACCOUNT_TOP_UP_V1: &str = "account_top_up_v1";
pub const BLOB_STORAGE_CASHIER_STORAGE_GATEWAY_PRINCIPAL_LIST_V1: &str =
    "storage_gateway_principal_list_v1";

pub const CANIC_SYNC_STATE: &str = "canic_sync_state";
pub const CANIC_SYNC_TOPOLOGY: &str = "canic_sync_topology";

pub const CANIC_WASM_STORE_ROOT_UPDATE_METHODS: &[&str] = &[
    CANIC_WASM_STORE_BEGIN_GC,
    CANIC_WASM_STORE_CHUNK,
    CANIC_WASM_STORE_COMPLETE_GC,
    CANIC_WASM_STORE_INFO,
    CANIC_WASM_STORE_PREPARE,
    CANIC_WASM_STORE_PREPARE_GC,
    CANIC_WASM_STORE_PUBLISH_CHUNK,
    CANIC_WASM_STORE_STAGE_MANIFEST,
];

pub const CANIC_WASM_STORE_STRUCTURAL_QUERY_METHODS: &[&str] =
    &[CANIC_WASM_STORE_CATALOG, CANIC_WASM_STORE_STATUS];

pub const BLOB_STORAGE_069_GATEWAY_METHODS: &[&str] = &[
    BLOB_STORAGE_BLOBS_ARE_LIVE,
    BLOB_STORAGE_BLOBS_TO_DELETE,
    BLOB_STORAGE_CONFIRM_BLOB_DELETION,
    BLOB_STORAGE_CREATE_CERTIFICATE,
];

pub const BLOB_STORAGE_070_GATEWAY_METHODS: &[&str] = &[
    BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
    BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
];

pub const BLOB_STORAGE_070_CASHIER_METHODS: &[&str] = &[
    BLOB_STORAGE_CASHIER_ACCOUNT_BALANCE_GET_V1,
    BLOB_STORAGE_CASHIER_ACCOUNT_TOP_UP_V1,
    BLOB_STORAGE_CASHIER_STORAGE_GATEWAY_PRINCIPAL_LIST_V1,
];
