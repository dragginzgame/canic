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
    env::{APP_STATE_ID, ENV_ID, SUBNET_STATE_ID},
    intent::{INTENT_META_ID, INTENT_PENDING_ID, INTENT_RECORDS_ID, INTENT_TOTALS_ID},
    observability::{
        CYCLE_TOPUP_EVENTS_ID, CYCLE_TRACKER_ID, CYCLES_FUNDING_LEDGER_ID, ICP_REFILL_RECORDS_ID,
        LOG_DATA_ID, LOG_INDEX_ID,
    },
    placement::{DIRECTORY_REGISTRY_ID, SCALING_REGISTRY_ID},
    pool::CANISTER_POOL_ID,
    topology::{
        APP_INDEX_ID, APP_REGISTRY_ID, CANISTER_CHILDREN_ID, SUBNET_INDEX_ID, SUBNET_REGISTRY_ID,
    },
};

#[cfg(feature = "blob-storage-billing")]
use crate::storage::stable::memory::blob_storage::BLOB_STORAGE_BILLING_ID;
#[cfg(feature = "blob-storage")]
use crate::storage::stable::memory::blob_storage::{
    BLOB_DELETION_PENDING_ID, STORAGE_GATEWAY_PRINCIPALS_ID, STORED_BLOBS_ID,
};
#[cfg(feature = "sharding")]
use crate::storage::stable::memory::placement::{
    SHARDING_ACTIVE_SET_ID, SHARDING_ASSIGNMENT_ID, SHARDING_REGISTRY_ID,
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
    pub reserved_memory: Vec<ReservedMemoryManifest>,
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
    NotApplicable,
}

impl StateStorage {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StableMemory => "stable_memory",
            Self::HeapOnly => "heap_only",
            Self::NotApplicable => "not_applicable",
        }
    }
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

impl MigrationPolicy {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NewDomain => "new_domain",
            Self::Migrate => "migrate",
            Self::ManualMigrationRequired => "manual_migration_required",
            Self::DiscardDeclared => "discard_declared",
            Self::NotApplicable => "not_applicable",
        }
    }
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

///
/// ReservedMemoryManifest
///
/// Explicit reservation for a stable memory ID whose persisted state shape is
/// known but not yet represented as one active or removed state domain.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReservedMemoryManifest {
    pub label: String,
    pub memory_id: u8,
    pub owner: String,
    pub reason: String,
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
    let Some(role) = role else {
        return canic_state_manifest();
    };
    if role == ROOT_ROLE {
        return canic_state_manifest();
    }

    let mut manifest = StateManifest {
        schema_version: STATE_MANIFEST_SCHEMA_VERSION,
        roles: feature_role_manifest(role).into_iter().collect(),
    };
    sort_manifest(&mut manifest);
    manifest
}

fn feature_role_manifest(role: &str) -> Option<StateRoleManifest> {
    let reserved_memory = feature_reserved_memory_domains();
    if reserved_memory.is_empty() {
        return None;
    }

    Some(StateRoleManifest {
        canister_role: role.to_string(),
        state: Vec::new(),
        removed_state: Vec::new(),
        reserved_memory,
    })
}

fn feature_reserved_memory_domains() -> Vec<ReservedMemoryManifest> {
    let mut reserved = sharding_reserved_memory_domains();
    reserved.extend(blob_storage_reserved_memory_domains());
    reserved
}

#[cfg(feature = "sharding")]
fn sharding_reserved_memory_domains() -> Vec<ReservedMemoryManifest> {
    vec![
        reserved_memory(
            "sharding_registry",
            SHARDING_REGISTRY_ID,
            "feature-gated sharding registry needs a precise role-specific Data snapshot declaration",
        ),
        reserved_memory(
            "sharding_assignments",
            SHARDING_ASSIGNMENT_ID,
            "feature-gated sharding assignments need a precise role-specific record and Data snapshot declaration",
        ),
        reserved_memory(
            "sharding_active_set",
            SHARDING_ACTIVE_SET_ID,
            "feature-gated sharding lifecycle state needs a precise role-specific Data snapshot declaration",
        ),
    ]
}

#[cfg(not(feature = "sharding"))]
const fn sharding_reserved_memory_domains() -> Vec<ReservedMemoryManifest> {
    Vec::new()
}

#[cfg(feature = "blob-storage")]
fn blob_storage_reserved_memory_domains() -> Vec<ReservedMemoryManifest> {
    let mut reserved = vec![
        reserved_memory(
            "stored_blobs",
            STORED_BLOBS_ID,
            "feature-gated blob storage needs a precise role-specific domain declaration",
        ),
        reserved_memory(
            "blob_deletion_pending",
            BLOB_DELETION_PENDING_ID,
            "feature-gated blob deletion state needs a precise role-specific domain declaration",
        ),
        reserved_memory(
            "storage_gateway_principals",
            STORAGE_GATEWAY_PRINCIPALS_ID,
            "feature-gated gateway state needs a precise role-specific domain declaration",
        ),
    ];

    #[cfg(feature = "blob-storage-billing")]
    reserved.push(reserved_memory(
        "blob_storage_billing",
        BLOB_STORAGE_BILLING_ID,
        "feature-gated blob billing state needs a precise role-specific Data snapshot declaration",
    ));

    reserved
}

#[cfg(not(feature = "blob-storage"))]
const fn blob_storage_reserved_memory_domains() -> Vec<ReservedMemoryManifest> {
    Vec::new()
}

fn root_role_manifest() -> StateRoleManifest {
    let mut state = Vec::new();
    state.extend(root_topology_domains());
    state.extend(root_env_domains());
    state.extend(root_auth_domains());
    state.extend(root_observability_domains());
    state.extend(root_intent_domains());
    state.extend(root_capacity_domains());

    StateRoleManifest {
        canister_role: ROOT_ROLE.to_string(),
        state,
        removed_state: root_removed_state_domains(),
        reserved_memory: root_reserved_memory_domains(),
    }
}

fn root_topology_domains() -> Vec<StateDomainManifest> {
    vec![
        state_domain(
            "app_index",
            APP_INDEX_ID,
            "AppIndexRecord",
            "AppIndexData",
            10,
            "app_index_import_restores_unique_roles",
        ),
        state_domain(
            "subnet_index",
            SUBNET_INDEX_ID,
            "SubnetIndexRecord",
            "SubnetIndexData",
            15,
            "subnet_index_import_restores_unique_roles",
        ),
        state_domain(
            "app_registry",
            APP_REGISTRY_ID,
            "AppRegistryRecord",
            "AppRegistryData",
            20,
            "app_registry_entries_have_root_principals",
        ),
        state_domain(
            "subnet_registry",
            SUBNET_REGISTRY_ID,
            "SubnetRegistryRecord",
            "SubnetRegistryData",
            25,
            "subnet_registry_parent_links_are_restored",
        ),
        state_domain(
            "canister_children",
            CANISTER_CHILDREN_ID,
            "CanisterChildrenRecord",
            "CanisterChildrenData",
            30,
            "canister_children_projection_is_imported",
        ),
    ]
}

fn root_env_domains() -> Vec<StateDomainManifest> {
    vec![
        state_domain(
            "env",
            ENV_ID,
            "EnvRecord",
            "EnvData",
            40,
            "env_root_and_role_bindings_are_restored",
        ),
        state_domain(
            "app_state",
            APP_STATE_ID,
            "AppStateRecord",
            "AppStateData",
            50,
            "app_state_mode_is_restored_before_hooks",
        ),
        state_domain(
            "subnet_state",
            SUBNET_STATE_ID,
            "SubnetStateRecord",
            "SubnetStateData",
            55,
            "subnet_state_restores_auth_state",
        ),
    ]
}

fn root_auth_domains() -> Vec<StateDomainManifest> {
    vec![
        state_domain(
            "auth_state",
            AUTH_STATE_ID,
            "AuthStateRecord",
            "AuthStateData",
            60,
            "auth_state_delegated_proofs_are_chain_key_only",
        ),
        state_domain(
            "replay_receipts",
            REPLAY_RECEIPTS_ID,
            "ReplayReceiptRecord",
            "ReplayReceiptData",
            70,
            "replay_receipts_reject_unsupported_schema_versions",
        ),
    ]
}

fn root_observability_domains() -> Vec<StateDomainManifest> {
    vec![
        state_domain(
            "cycle_topup_events",
            CYCLE_TOPUP_EVENTS_ID,
            "CycleTopupEventRecord",
            "CycleTopupEventsData",
            80,
            "cycle_topup_events_decode_status_values",
        ),
        state_domain(
            "cycles_funding_ledger",
            CYCLES_FUNDING_LEDGER_ID,
            "CyclesFundingLedgerRecord",
            "CyclesFundingLedgerData",
            90,
            "cycles_funding_ledger_restores_child_budget_state",
        ),
        state_domain(
            "icp_refill_records",
            ICP_REFILL_RECORDS_ID,
            "IcpRefillRecord",
            "IcpRefillRecordsData",
            100,
            "icp_refill_records_decode_status_and_error_codes",
        ),
    ]
}

fn root_intent_domains() -> Vec<StateDomainManifest> {
    vec![
        state_domain(
            "intent_meta",
            INTENT_META_ID,
            "IntentStoreMetaRecord",
            "IntentMetaData",
            110,
            "intent_meta_restores_schema_version",
        ),
        state_domain(
            "intent_records",
            INTENT_RECORDS_ID,
            "IntentRecord",
            "IntentRecordsData",
            111,
            "intent_records_restore_state_transitions",
        ),
        state_domain(
            "intent_totals",
            INTENT_TOTALS_ID,
            "IntentResourceTotalsRecord",
            "IntentTotalsData",
            112,
            "intent_totals_restore_resource_accounting",
        ),
        state_domain(
            "intent_pending",
            INTENT_PENDING_ID,
            "IntentPendingEntryRecord",
            "IntentPendingData",
            113,
            "intent_pending_entries_restore_ttl_metadata",
        ),
    ]
}

fn root_capacity_domains() -> Vec<StateDomainManifest> {
    vec![
        state_domain(
            "canister_pool",
            CANISTER_POOL_ID,
            "PoolStoreRecord",
            "CanisterPoolData",
            130,
            "canister_pool_entries_restore_header_state",
        ),
        state_domain(
            "scaling_registry",
            SCALING_REGISTRY_ID,
            "ScalingRegistryRecord",
            "ScalingRegistryData",
            140,
            "scaling_registry_restores_worker_pool_membership",
        ),
        state_domain(
            "directory_registry",
            DIRECTORY_REGISTRY_ID,
            "DirectoryRegistryRecord",
            "DirectoryRegistryData",
            150,
            "directory_registry_entries_restore_bindings",
        ),
    ]
}

fn root_removed_state_domains() -> Vec<RemovedStateManifest> {
    vec![RemovedStateManifest {
        domain: "root_replay".to_string(),
        last_version: 1,
        removed_in_version: 2,
        memory_id: Some(ROOT_REPLAY_ID),
        disposition: "moved_to_replay_receipts".to_string(),
        reason: "root replay receipts moved into the shared replay receipt store".to_string(),
        test: Some("replay_receipt_record_round_trips_through_cbor_storable".to_string()),
    }]
}

fn root_reserved_memory_domains() -> Vec<ReservedMemoryManifest> {
    vec![
        reserved_memory(
            "cycle_tracker",
            CYCLE_TRACKER_ID,
            "cycle tracker stores raw cycle balances and needs an explicit record/snapshot declaration",
        ),
        reserved_memory(
            "log_index",
            LOG_INDEX_ID,
            "stable log index memory is one half of the logical log domain and needs multi-memory domain modeling",
        ),
        reserved_memory(
            "log_data",
            LOG_DATA_ID,
            "stable log data memory is one half of the logical log domain and needs multi-memory domain modeling",
        ),
    ]
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

fn reserved_memory(label: &str, memory_id: u8, reason: &str) -> ReservedMemoryManifest {
    ReservedMemoryManifest {
        label: label.to_string(),
        memory_id,
        owner: CANIC_CORE_OWNER.to_string(),
        reason: reason.to_string(),
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
        role.reserved_memory
            .sort_by_key(|reservation| reservation.memory_id);
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
    fn state_contract_enums_own_serialized_labels() {
        assert_eq!(StateStorage::StableMemory.as_str(), "stable_memory");
        assert_eq!(StateStorage::HeapOnly.as_str(), "heap_only");
        assert_eq!(StateStorage::NotApplicable.as_str(), "not_applicable");
        assert_eq!(MigrationPolicy::NewDomain.as_str(), "new_domain");
        assert_eq!(MigrationPolicy::Migrate.as_str(), "migrate");
        assert_eq!(
            MigrationPolicy::ManualMigrationRequired.as_str(),
            "manual_migration_required"
        );
        assert_eq!(
            MigrationPolicy::DiscardDeclared.as_str(),
            "discard_declared"
        );
        assert_eq!(MigrationPolicy::NotApplicable.as_str(), "not_applicable");
    }

    #[test]
    fn root_manifest_covers_declared_core_memory_ids() {
        let manifest = canic_state_manifest_for_role(Some(ROOT_ROLE));
        let role = manifest.roles.first().expect("root role manifest");
        let ids = role
            .state
            .iter()
            .filter_map(|domain| domain.memory_id)
            .collect::<Vec<_>>();

        for expected in [
            CANISTER_CHILDREN_ID,
            APP_INDEX_ID,
            SUBNET_INDEX_ID,
            APP_REGISTRY_ID,
            SUBNET_REGISTRY_ID,
            ENV_ID,
            SUBNET_STATE_ID,
            APP_STATE_ID,
            AUTH_STATE_ID,
            REPLAY_RECEIPTS_ID,
            CYCLE_TOPUP_EVENTS_ID,
            ICP_REFILL_RECORDS_ID,
            CYCLES_FUNDING_LEDGER_ID,
            INTENT_META_ID,
            INTENT_RECORDS_ID,
            INTENT_TOTALS_ID,
            INTENT_PENDING_ID,
            CANISTER_POOL_ID,
            SCALING_REGISTRY_ID,
            DIRECTORY_REGISTRY_ID,
        ] {
            assert!(
                ids.contains(&expected),
                "state manifest should declare memory id {expected}"
            );
        }
    }

    #[test]
    fn root_manifest_tracks_reserved_core_memory_ids() {
        let manifest = canic_state_manifest_for_role(Some(ROOT_ROLE));
        let role = manifest.roles.first().expect("root role manifest");
        let ids = role
            .reserved_memory
            .iter()
            .map(|reservation| reservation.memory_id)
            .collect::<Vec<_>>();

        for expected in [CYCLE_TRACKER_ID, LOG_INDEX_ID, LOG_DATA_ID] {
            assert!(
                ids.contains(&expected),
                "state manifest should reserve memory id {expected}"
            );
        }
    }

    #[test]
    fn role_filter_returns_empty_manifest_for_unknown_role() {
        let manifest = canic_state_manifest_for_role(Some("unknown"));

        if cfg!(any(feature = "sharding", feature = "blob-storage")) {
            assert_eq!(manifest.roles.len(), 1);
            assert_eq!(manifest.roles[0].canister_role, "unknown");
            assert!(manifest.roles[0].state.is_empty());
            assert!(!manifest.roles[0].reserved_memory.is_empty());
        } else {
            assert!(manifest.roles.is_empty());
        }
    }

    #[cfg(any(feature = "sharding", feature = "blob-storage"))]
    #[test]
    fn feature_role_manifest_reserves_every_feature_gated_memory_id() {
        let manifest = canic_state_manifest_for_role(Some("feature_role"));
        let role = manifest.roles.first().expect("feature role manifest");
        let ids = role
            .reserved_memory
            .iter()
            .map(|reservation| reservation.memory_id)
            .collect::<Vec<_>>();

        #[cfg(feature = "sharding")]
        for expected in [
            SHARDING_REGISTRY_ID,
            SHARDING_ASSIGNMENT_ID,
            SHARDING_ACTIVE_SET_ID,
        ] {
            assert!(ids.contains(&expected));
        }

        #[cfg(feature = "blob-storage")]
        for expected in [
            STORED_BLOBS_ID,
            BLOB_DELETION_PENDING_ID,
            STORAGE_GATEWAY_PRINCIPALS_ID,
        ] {
            assert!(ids.contains(&expected));
        }

        #[cfg(feature = "blob-storage-billing")]
        assert!(ids.contains(&BLOB_STORAGE_BILLING_ID));
    }
}
