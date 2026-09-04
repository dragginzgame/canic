/// Runtime wire-level endpoint names used by `canic-core` for inter-canister calls.
///
/// Keep these synchronized with the macro-defined endpoints.

pub const CANIC_COMMAND: &str = "canic_command";
pub const CANIC_COORDINATOR_COMMAND: &str = "canic_coordinator_command";
pub const CANIC_COORDINATOR_STATUS: &str = "canic_coordinator_status";
pub const CANIC_ROOT_COMMAND: &str = "canic_root_command";
pub const CANIC_ROOT_STATUS: &str = "canic_root_status";
pub const CANIC_STATUS: &str = "canic_status";
pub const CANIC_WASM_STORE_COMMAND: &str = "canic_wasm_store_command";
pub const CANIC_WASM_STORE_STATUS: &str = "canic_wasm_store_status";

/// Return the exact command endpoint owned by one Canic role.
#[must_use]
pub fn command_endpoint_for_role(role: &crate::ids::CanisterRole) -> &'static str {
    if role.is_fleet_coordinator() {
        CANIC_COORDINATOR_COMMAND
    } else if role.is_root() {
        CANIC_ROOT_COMMAND
    } else if role.is_wasm_store() {
        CANIC_WASM_STORE_COMMAND
    } else {
        CANIC_COMMAND
    }
}

/// Return the exact status endpoint owned by one Canic role.
#[must_use]
pub fn status_endpoint_for_role(role: &crate::ids::CanisterRole) -> &'static str {
    if role.is_fleet_coordinator() {
        CANIC_COORDINATOR_STATUS
    } else if role.is_root() {
        CANIC_ROOT_STATUS
    } else if role.is_wasm_store() {
        CANIC_WASM_STORE_STATUS
    } else {
        CANIC_STATUS
    }
}

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

#[cfg(test)]
mod tests {
    use super::{
        CANIC_COMMAND, CANIC_COORDINATOR_COMMAND, CANIC_COORDINATOR_STATUS, CANIC_ROOT_COMMAND,
        CANIC_ROOT_STATUS, CANIC_STATUS, CANIC_WASM_STORE_COMMAND, CANIC_WASM_STORE_STATUS,
        command_endpoint_for_role, status_endpoint_for_role,
    };
    use crate::ids::CanisterRole;
    use std::collections::BTreeSet;

    #[test]
    fn built_in_roles_own_distinct_typed_command_and_status_endpoints() {
        let ordinary = CanisterRole::new("ordinary");
        let roles = [
            (&ordinary, CANIC_COMMAND, CANIC_STATUS),
            (
                &CanisterRole::FLEET_COORDINATOR,
                CANIC_COORDINATOR_COMMAND,
                CANIC_COORDINATOR_STATUS,
            ),
            (&CanisterRole::ROOT, CANIC_ROOT_COMMAND, CANIC_ROOT_STATUS),
            (
                &CanisterRole::WASM_STORE,
                CANIC_WASM_STORE_COMMAND,
                CANIC_WASM_STORE_STATUS,
            ),
        ];

        for (role, command, status) in roles {
            assert_eq!(command_endpoint_for_role(role), command);
            assert_eq!(status_endpoint_for_role(role), status);
        }

        let commands = roles
            .iter()
            .map(|(_, command, _)| *command)
            .collect::<BTreeSet<_>>();
        assert_eq!(commands.len(), roles.len());

        let statuses = roles
            .iter()
            .map(|(_, _, status)| *status)
            .collect::<BTreeSet<_>>();
        assert_eq!(statuses.len(), roles.len());
    }
}
