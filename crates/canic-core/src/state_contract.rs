//! Module: state_contract
//!
//! Responsibility: declare Canic-owned stable state metadata for host-side
//! state manifest and audit reports.
//! Does not own: CLI rendering, migration execution, stable-memory reads, or
//! stable-memory writes.
//! Boundary: declarations are static Rust metadata derived from the storage
//! modules that own the records and memory IDs.

use serde::Serialize;

use crate::storage::stable::memory::{
    auth::{AUTH_STATE_ID, REPLAY_RECEIPTS_ID, ROOT_REPLAY_ID},
    env::{APP_STATE_ID, ENV_ID},
    topology::{APP_INDEX_ID, APP_REGISTRY_ID, CANISTER_CHILDREN_ID},
};

pub const STATE_MANIFEST_SCHEMA_VERSION: u16 = 1;
const ROOT_ROLE: &str = "root";
const CANIC_CORE_OWNER: &str = "canic-core";

///
/// StateManifest
///
/// Derived state manifest rendered by host tooling.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StateManifest {
    pub schema_version: u16,
    pub roles: Vec<StateRoleManifest>,
}

///
/// StateRoleManifest
///
/// Declared state domains for one canister role.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StateRoleManifest {
    pub canister_role: String,
    pub state: Vec<StateDomainManifest>,
    pub removed_state: Vec<RemovedStateManifest>,
}

///
/// StateDomainManifest
///
/// Static declaration for one active state domain.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StateDomainManifest {
    pub domain: String,
    pub version: u32,
    pub storage: StateStorage,
    pub memory_id: Option<u8>,
    pub owner: String,
    pub record: String,
    pub snapshot: String,
    pub min_supported_version: u32,
    pub migration_policy: MigrationPolicy,
    pub restore_order: Option<u32>,
    pub post_upgrade_invariant: Option<String>,
    pub migrations: Vec<StateMigrationManifest>,
}

///
/// StateStorage
///
/// Persistence substrate declared for a state domain.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateStorage {
    StableMemory,
    HeapOnly,
}

///
/// MigrationPolicy
///
/// Declared upgrade policy for the domain's supported version window.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationPolicy {
    NewDomain,
    Migrate,
    ManualMigrationRequired,
    DiscardDeclared,
    NotApplicable,
}

///
/// StateMigrationManifest
///
/// Declared migration or migration coverage metadata.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StateMigrationManifest {
    pub from: u32,
    pub to: u32,
    pub kind: String,
    pub name: Option<String>,
    pub test: Option<String>,
}

///
/// RemovedStateManifest
///
/// Explicit disposition for a retired state domain or memory ID.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RemovedStateManifest {
    pub domain: String,
    pub last_version: u32,
    pub removed_in_version: u32,
    pub memory_id: Option<u8>,
    pub disposition: String,
    pub reason: String,
    pub test: Option<String>,
}

#[must_use]
pub fn canic_state_manifest() -> StateManifest {
    let mut manifest = StateManifest {
        schema_version: STATE_MANIFEST_SCHEMA_VERSION,
        roles: vec![root_role_manifest()],
    };
    sort_manifest(&mut manifest);
    manifest
}

#[must_use]
pub fn canic_state_manifest_for_role(role: Option<&str>) -> StateManifest {
    let mut manifest = canic_state_manifest();
    if let Some(role) = role {
        manifest
            .roles
            .retain(|entry| entry.canister_role.as_str() == role);
    }
    manifest
}

fn root_role_manifest() -> StateRoleManifest {
    StateRoleManifest {
        canister_role: ROOT_ROLE.to_string(),
        state: vec![
            state_domain(
                "app_index",
                APP_INDEX_ID,
                "AppIndexRecord",
                "AppIndexRecord",
                10,
                "app_index_import_restores_unique_roles",
            ),
            state_domain(
                "app_registry",
                APP_REGISTRY_ID,
                "AppRegistryRecord",
                "AppRegistryRecord",
                20,
                "app_registry_entries_have_root_principals",
            ),
            state_domain(
                "auth_state",
                AUTH_STATE_ID,
                "AuthStateRecord",
                "AuthStateRecord",
                60,
                "auth_state_delegated_proofs_are_chain_key_only",
            ),
            state_domain(
                "canister_children",
                CANISTER_CHILDREN_ID,
                "CanisterChildrenRecord",
                "CanisterChildrenRecord",
                30,
                "canister_children_projection_is_imported",
            ),
            state_domain(
                "env",
                ENV_ID,
                "EnvRecord",
                "EnvRecord",
                40,
                "env_root_and_role_bindings_are_restored",
            ),
            state_domain(
                "app_state",
                APP_STATE_ID,
                "AppStateRecord",
                "AppStateRecord",
                50,
                "app_state_mode_is_restored_before_hooks",
            ),
            state_domain(
                "replay_receipts",
                REPLAY_RECEIPTS_ID,
                "ReplayReceiptRecord",
                "ReplayReceiptRecord",
                70,
                "replay_receipts_reject_unsupported_schema_versions",
            ),
        ],
        removed_state: vec![RemovedStateManifest {
            domain: "root_replay".to_string(),
            last_version: 1,
            removed_in_version: 2,
            memory_id: Some(ROOT_REPLAY_ID),
            disposition: "moved_to_replay_receipts".to_string(),
            reason: "root replay receipts moved into the shared replay receipt store".to_string(),
            test: Some("root_replay_record_round_trips_populated_response".to_string()),
        }],
    }
}

fn state_domain(
    domain: &str,
    memory_id: u8,
    record: &str,
    snapshot: &str,
    restore_order: u32,
    invariant: &str,
) -> StateDomainManifest {
    StateDomainManifest {
        domain: domain.to_string(),
        version: 1,
        storage: StateStorage::StableMemory,
        memory_id: Some(memory_id),
        owner: CANIC_CORE_OWNER.to_string(),
        record: record.to_string(),
        snapshot: snapshot.to_string(),
        min_supported_version: 1,
        migration_policy: MigrationPolicy::NewDomain,
        restore_order: Some(restore_order),
        post_upgrade_invariant: Some(invariant.to_string()),
        migrations: Vec::new(),
    }
}

fn sort_manifest(manifest: &mut StateManifest) {
    manifest
        .roles
        .sort_by(|left, right| left.canister_role.cmp(&right.canister_role));
    for role in &mut manifest.roles {
        role.state
            .sort_by(|left, right| left.domain.cmp(&right.domain));
        role.removed_state
            .sort_by(|left, right| left.domain.cmp(&right.domain));
        for domain in &mut role.state {
            domain
                .migrations
                .sort_by_key(|migration| (migration.from, migration.to));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_manifest_uses_unique_memory_ids() {
        let manifest = canic_state_manifest_for_role(Some(ROOT_ROLE));
        let role = manifest.roles.first().expect("root role manifest");
        let mut ids = role
            .state
            .iter()
            .filter_map(|domain| domain.memory_id)
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), role.state.len());
    }

    #[test]
    fn role_filter_returns_empty_manifest_for_unknown_role() {
        let manifest = canic_state_manifest_for_role(Some("unknown"));
        assert!(manifest.roles.is_empty());
    }
}
