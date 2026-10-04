/// Runtime wire-level endpoint names used by `canic-core` for inter-canister calls.
///
/// Keep these synchronized with the macro-defined endpoints.

pub const CANIC_COMMAND: &str = "canic_command";
pub const CANIC_COORDINATOR_COMMAND: &str = "canic_coordinator_command";
pub const CANIC_COORDINATOR_OPERATION_STATUS: &str = "canic_coordinator_operation_status";
pub const CANIC_COORDINATOR_REGISTRY: &str = "canic_coordinator_registry";
pub const CANIC_WASM_STORE_CATALOG: &str = "canic_wasm_store_catalog";
pub const CANIC_ROOT_COMMAND: &str = "canic_root_command";
pub const CANIC_ROOT_FIXTURE_STATUS: &str = "canic_root_fixture_status";
pub const CANIC_ROOT_STATUS: &str = "canic_root_status";
pub const CANIC_PUBLIC_STATUS: &str = "canic_public_status";
pub const CANIC_OBSERVABILITY: &str = "canic_observability";
pub const CANIC_AUTH_STATUS: &str = "canic_auth_status";
pub const CANIC_CONTROL_STATUS: &str = "canic_control_status";
pub const CANIC_ADMISSION_STATUS: &str = "canic_admission_status";
pub const CANIC_ROOT_AUTH_STATUS: &str = "canic_root_auth_status";
pub const CANIC_ROOT_OPERATION_STATUS: &str = "canic_root_operation_status";
pub const CANIC_WASM_STORE_COMMAND: &str = "canic_wasm_store_command";
pub const CANIC_WASM_STORE_STATUS: &str = "canic_wasm_store_status";
pub const CANIC_WASM_STORE_FIXTURE_CHUNK: &str = "canic_wasm_store_fixture_chunk";
pub const CANIC_WASM_STORE_PUBLISH_FIXTURE: &str = "canic_wasm_store_publish_fixture";

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

/// Maximum encoded payload accepted by state and topology cascade endpoints.
pub const CASCADE_SNAPSHOT_MAX_BYTES: usize = 16_384;

#[cfg(test)]
mod tests {
    use super::{
        CANIC_COMMAND, CANIC_COORDINATOR_COMMAND, CANIC_ROOT_COMMAND, CANIC_WASM_STORE_COMMAND,
        command_endpoint_for_role,
    };
    use crate::ids::CanisterRole;
    use std::collections::BTreeSet;

    #[test]
    fn built_in_roles_own_distinct_command_endpoints() {
        let ordinary = CanisterRole::new("ordinary");
        let roles = [
            (&ordinary, CANIC_COMMAND),
            (&CanisterRole::FLEET_COORDINATOR, CANIC_COORDINATOR_COMMAND),
            (&CanisterRole::ROOT, CANIC_ROOT_COMMAND),
            (&CanisterRole::WASM_STORE, CANIC_WASM_STORE_COMMAND),
        ];
        for (role, command) in roles {
            assert_eq!(command_endpoint_for_role(role), command);
        }
        let commands = roles
            .iter()
            .map(|(_, command)| *command)
            .collect::<BTreeSet<_>>();
        assert_eq!(commands.len(), roles.len());
    }
}
