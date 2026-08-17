/// Runtime wire-level endpoint names used by `canic-core` for inter-canister calls.
///
/// Keep these synchronized with the macro-defined endpoints.

pub const CANIC_COMMAND: &str = "canic_command";
pub const CANIC_STATUS: &str = "canic_status";
pub const CANIC_WASM_STORE_CHUNK: &str = "canic_wasm_store_chunk";
pub const CANIC_WASM_STORE_PUBLISH_CHUNK: &str = "canic_wasm_store_publish_chunk";

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

/// Maximum encoded payload accepted by state and topology cascade endpoints.
pub const CASCADE_SNAPSHOT_MAX_BYTES: usize = 16_384;

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
