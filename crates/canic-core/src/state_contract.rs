//! Module: state_contract
//!
//! Responsibility: declare Canic-owned stable state metadata for host-side
//! state manifest and audit reports.
//! Does not own: CLI rendering, migration execution, stable-memory reads, or
//! stable-memory writes.
//! Boundary: declarations are static Rust metadata derived from the storage
//! modules that own the records and memory IDs.

use serde::Serialize;

use crate::role_contract::allocation::memory::{
    auth::{AUTH_STATE_ID, REPLAY_RECEIPTS_ID},
    blob_storage::{
        BLOB_DELETION_PENDING_ID, BLOB_STORAGE_BILLING_ID, STORAGE_GATEWAY_PRINCIPALS_ID,
        STORED_BLOBS_ID,
    },
    env::{APP_STATE_ID, ENV_ID, SUBNET_STATE_ID},
    intent::{INTENT_META_ID, INTENT_PENDING_ID, INTENT_RECORDS_ID, INTENT_TOTALS_ID},
    observability::{
        CYCLE_TOPUP_EVENTS_ID, CYCLE_TRACKER_ID, CYCLES_FUNDING_LEDGER_ID, ICP_REFILL_RECORDS_ID,
        LOG_DATA_ID, LOG_INDEX_ID,
    },
    placement::{
        DIRECTORY_REGISTRY_ID, SCALING_REGISTRY_ID, SHARDING_ACTIVE_SET_ID, SHARDING_ASSIGNMENT_ID,
        SHARDING_REGISTRY_ID,
    },
    pool::CANISTER_POOL_ID,
    topology::{
        APP_INDEX_ID, APP_REGISTRY_ID, CANISTER_CHILDREN_ID, SUBNET_INDEX_ID, SUBNET_REGISTRY_ID,
    },
};
use crate::role_contract::{AllocationOwner, StateAllocationKey};

pub const STATE_MANIFEST_SCHEMA_VERSION: u16 = 2;

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
/// ReservedMemoryManifest
///
/// Explicit reservation for a stable memory ID whose persisted state shape is
/// known but not yet represented as one active state domain.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReservedMemoryManifest {
    pub label: String,
    pub memory_id: u8,
    pub owner: String,
    pub reason: String,
}

///
/// StateAllocationDescriptor
///
/// Owner-provided state metadata for one active allocation key.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateAllocationDescriptor {
    pub allocation: StateAllocationKey,
    pub owner: AllocationOwner,
    pub state: Vec<StateDomainManifest>,
    pub reserved_memory: Vec<ReservedMemoryManifest>,
}

#[must_use]
pub fn canic_state_descriptors() -> Vec<StateAllocationDescriptor> {
    let mut descriptors = core_runtime_descriptors();
    descriptors.extend(placement_capacity_descriptors());
    descriptors.extend(sharding_descriptors());
    descriptors.extend(blob_storage_descriptors());
    descriptors
}

fn core_runtime_descriptors() -> Vec<StateAllocationDescriptor> {
    vec![
        descriptor(
            StateAllocationKey::CoreRuntimeTopology,
            runtime_topology_domains(),
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::CoreRootAppRegistry,
            root_app_registry_domains(),
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::CoreRuntimeEnvironment,
            runtime_env_domains(),
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::CoreAuthState,
            auth_state_domains(),
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::CoreReplayReceipts,
            replay_receipt_domains(),
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::CoreRuntimeObservability,
            runtime_observability_domains(),
            runtime_reserved_memory_domains(),
        ),
        descriptor(
            StateAllocationKey::CoreIcpRefillRecords,
            icp_refill_domains(),
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::CoreRuntimeIntent,
            runtime_intent_domains(),
            Vec::new(),
        ),
    ]
}

fn placement_capacity_descriptors() -> Vec<StateAllocationDescriptor> {
    use crate::storage::stable::{
        directory::{DirectoryRegistryData, DirectoryRegistryEntryRecord},
        pool::{CanisterPoolData, CanisterPoolEntryRecord},
        scaling::{ScalingRegistryData, ScalingRegistryEntryRecord},
    };

    vec![
        descriptor(
            StateAllocationKey::CanisterPool,
            vec![state_domain(
                "canister_pool",
                CANISTER_POOL_ID,
                CanisterPoolEntryRecord::STATE_CONTRACT_NAME,
                CanisterPoolData::STATE_CONTRACT_NAME,
                130,
                "canister_pool_entries_restore_header_state",
            )],
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::ScalingRegistry,
            vec![state_domain(
                "scaling_registry",
                SCALING_REGISTRY_ID,
                ScalingRegistryEntryRecord::STATE_CONTRACT_NAME,
                ScalingRegistryData::STATE_CONTRACT_NAME,
                140,
                "scaling_registry_restores_worker_pool_membership",
            )],
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::DirectoryRegistry,
            vec![state_domain(
                "directory_registry",
                DIRECTORY_REGISTRY_ID,
                DirectoryRegistryEntryRecord::STATE_CONTRACT_NAME,
                DirectoryRegistryData::STATE_CONTRACT_NAME,
                150,
                "directory_registry_entries_restore_bindings",
            )],
            Vec::new(),
        ),
    ]
}

fn sharding_descriptors() -> Vec<StateAllocationDescriptor> {
    use crate::storage::stable::sharding::{
        ShardEntryRecord, ShardingActiveSetData, ShardingActiveSetRecord, ShardingAssignmentRecord,
        ShardingAssignmentsData, ShardingRegistryData,
    };

    vec![
        descriptor(
            StateAllocationKey::ShardingRegistry,
            vec![state_domain(
                "sharding_registry",
                SHARDING_REGISTRY_ID,
                ShardEntryRecord::STATE_CONTRACT_NAME,
                ShardingRegistryData::STATE_CONTRACT_NAME,
                160,
                "sharding_registry_restores_pool_membership",
            )],
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::ShardingAssignments,
            vec![state_domain(
                "sharding_assignments",
                SHARDING_ASSIGNMENT_ID,
                ShardingAssignmentRecord::STATE_CONTRACT_NAME,
                ShardingAssignmentsData::STATE_CONTRACT_NAME,
                170,
                "sharding_assignments_restore_partition_bindings",
            )],
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::ShardingActiveSet,
            vec![state_domain(
                "sharding_active_set",
                SHARDING_ACTIVE_SET_ID,
                ShardingActiveSetRecord::STATE_CONTRACT_NAME,
                ShardingActiveSetData::STATE_CONTRACT_NAME,
                180,
                "sharding_active_set_restores_active_shards",
            )],
            Vec::new(),
        ),
    ]
}

fn blob_storage_descriptors() -> Vec<StateAllocationDescriptor> {
    use crate::storage::stable::blob_storage::{
        BlobDeletionPendingData, BlobDeletionPendingRecord, BlobStorageBillingStateData,
        BlobStorageBillingStateRecord, StorageGatewayPrincipalRecord, StorageGatewayPrincipalsData,
        StoredBlobRecord, StoredBlobsData,
    };

    vec![
        descriptor(
            StateAllocationKey::StoredBlobs,
            vec![state_domain(
                "stored_blobs",
                STORED_BLOBS_ID,
                StoredBlobRecord::STATE_CONTRACT_NAME,
                StoredBlobsData::STATE_CONTRACT_NAME,
                190,
                "stored_blobs_restore_live_blob_roots",
            )],
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::BlobDeletionPending,
            vec![state_domain(
                "blob_deletion_pending",
                BLOB_DELETION_PENDING_ID,
                BlobDeletionPendingRecord::STATE_CONTRACT_NAME,
                BlobDeletionPendingData::STATE_CONTRACT_NAME,
                200,
                "blob_deletion_pending_restores_gateway_scrub_state",
            )],
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::StorageGatewayPrincipals,
            vec![state_domain(
                "storage_gateway_principals",
                STORAGE_GATEWAY_PRINCIPALS_ID,
                StorageGatewayPrincipalRecord::STATE_CONTRACT_NAME,
                StorageGatewayPrincipalsData::STATE_CONTRACT_NAME,
                210,
                "storage_gateway_principals_restore_authorized_gateways",
            )],
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::BlobStorageBilling,
            vec![state_domain(
                "blob_storage_billing",
                BLOB_STORAGE_BILLING_ID,
                BlobStorageBillingStateRecord::STATE_CONTRACT_NAME,
                BlobStorageBillingStateData::STATE_CONTRACT_NAME,
                220,
                "blob_storage_billing_restores_cashier_configuration",
            )],
            Vec::new(),
        ),
    ]
}

fn descriptor(
    allocation: StateAllocationKey,
    mut state: Vec<StateDomainManifest>,
    mut reserved_memory: Vec<ReservedMemoryManifest>,
) -> StateAllocationDescriptor {
    state.sort_by(|left, right| left.domain.cmp(&right.domain));
    reserved_memory.sort_by_key(|reservation| reservation.memory_id);
    StateAllocationDescriptor {
        allocation,
        owner: AllocationOwner::CanicCore,
        state,
        reserved_memory,
    }
}

fn runtime_topology_domains() -> Vec<StateDomainManifest> {
    use crate::storage::{
        canister::CanisterEntryRecord,
        stable::{
            children::CanisterChildrenData,
            index::{IndexEntryRecord, app::AppIndexData, subnet::SubnetIndexData},
            registry::subnet::SubnetRegistryData,
        },
    };

    vec![
        state_domain(
            "app_index",
            APP_INDEX_ID,
            IndexEntryRecord::STATE_CONTRACT_NAME,
            AppIndexData::STATE_CONTRACT_NAME,
            10,
            "app_index_import_restores_unique_roles",
        ),
        state_domain(
            "subnet_index",
            SUBNET_INDEX_ID,
            IndexEntryRecord::STATE_CONTRACT_NAME,
            SubnetIndexData::STATE_CONTRACT_NAME,
            15,
            "subnet_index_import_restores_unique_roles",
        ),
        state_domain(
            "subnet_registry",
            SUBNET_REGISTRY_ID,
            CanisterEntryRecord::STATE_CONTRACT_NAME,
            SubnetRegistryData::STATE_CONTRACT_NAME,
            25,
            "subnet_registry_parent_links_are_restored",
        ),
        state_domain(
            "canister_children",
            CANISTER_CHILDREN_ID,
            CanisterEntryRecord::STATE_CONTRACT_NAME,
            CanisterChildrenData::STATE_CONTRACT_NAME,
            30,
            "canister_children_projection_is_imported",
        ),
    ]
}

fn root_app_registry_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::registry::app::{AppRegistryData, AppRegistryEntryRecord};

    vec![state_domain(
        "app_registry",
        APP_REGISTRY_ID,
        AppRegistryEntryRecord::STATE_CONTRACT_NAME,
        AppRegistryData::STATE_CONTRACT_NAME,
        20,
        "app_registry_entries_have_root_principals",
    )]
}

fn runtime_env_domains() -> Vec<StateDomainManifest> {
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

fn auth_state_domains() -> Vec<StateDomainManifest> {
    vec![state_domain(
        "auth_state",
        AUTH_STATE_ID,
        "AuthStateRecord",
        "AuthStateData",
        60,
        "auth_state_delegated_proofs_are_chain_key_only",
    )]
}

fn replay_receipt_domains() -> Vec<StateDomainManifest> {
    vec![state_domain(
        "replay_receipts",
        REPLAY_RECEIPTS_ID,
        "ReplayReceiptRecord",
        "ReplayReceiptData",
        70,
        "replay_receipts_reject_unsupported_schema_versions",
    )]
}

fn runtime_observability_domains() -> Vec<StateDomainManifest> {
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
    ]
}

fn icp_refill_domains() -> Vec<StateDomainManifest> {
    vec![state_domain(
        "icp_refill_records",
        ICP_REFILL_RECORDS_ID,
        "IcpRefillRecord",
        "IcpRefillRecordsData",
        100,
        "icp_refill_records_decode_status_and_error_codes",
    )]
}

fn runtime_intent_domains() -> Vec<StateDomainManifest> {
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

fn runtime_reserved_memory_domains() -> Vec<ReservedMemoryManifest> {
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
        owner: AllocationOwner::CanicCore.as_str().to_string(),
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
        owner: AllocationOwner::CanicCore.as_str().to_string(),
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptors_use_unique_memory_ids() {
        let descriptors = canic_state_descriptors();
        let mut ids = descriptors
            .iter()
            .flat_map(|descriptor| {
                descriptor
                    .state
                    .iter()
                    .filter_map(|domain| domain.memory_id)
                    .chain(
                        descriptor
                            .reserved_memory
                            .iter()
                            .map(|reservation| reservation.memory_id),
                    )
            })
            .collect::<Vec<_>>();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), count);
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
    fn descriptors_cover_declared_core_memory_ids() {
        let descriptors = canic_state_descriptors();
        let ids = descriptors
            .iter()
            .flat_map(|descriptor| descriptor.state.iter())
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
            SHARDING_REGISTRY_ID,
            SHARDING_ASSIGNMENT_ID,
            SHARDING_ACTIVE_SET_ID,
            STORED_BLOBS_ID,
            BLOB_DELETION_PENDING_ID,
            STORAGE_GATEWAY_PRINCIPALS_ID,
            BLOB_STORAGE_BILLING_ID,
        ] {
            assert!(
                ids.contains(&expected),
                "state manifest should declare memory id {expected}"
            );
        }
    }

    #[test]
    fn topology_index_descriptors_reference_canonical_data_types() {
        use crate::storage::stable::index::{
            IndexEntryRecord, app::AppIndexData, subnet::SubnetIndexData,
        };

        let descriptors = canic_state_descriptors();
        let topology = descriptors
            .iter()
            .find(|descriptor| descriptor.allocation == StateAllocationKey::CoreRuntimeTopology)
            .expect("runtime topology descriptor");

        for (domain, snapshot) in [
            ("app_index", AppIndexData::STATE_CONTRACT_NAME),
            ("subnet_index", SubnetIndexData::STATE_CONTRACT_NAME),
        ] {
            let declaration = topology
                .state
                .iter()
                .find(|declaration| declaration.domain == domain)
                .expect("topology index declaration");
            assert_eq!(declaration.record, IndexEntryRecord::STATE_CONTRACT_NAME);
            assert_eq!(declaration.snapshot, snapshot);
        }
    }

    #[test]
    fn topology_registry_descriptors_reference_canonical_data_types() {
        use crate::storage::{
            canister::CanisterEntryRecord,
            stable::{
                children::CanisterChildrenData,
                registry::{
                    app::{AppRegistryData, AppRegistryEntryRecord},
                    subnet::SubnetRegistryData,
                },
            },
        };

        let descriptors = canic_state_descriptors();

        for (allocation, domain, record, snapshot) in [
            (
                StateAllocationKey::CoreRuntimeTopology,
                "subnet_registry",
                CanisterEntryRecord::STATE_CONTRACT_NAME,
                SubnetRegistryData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CoreRuntimeTopology,
                "canister_children",
                CanisterEntryRecord::STATE_CONTRACT_NAME,
                CanisterChildrenData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CoreRootAppRegistry,
                "app_registry",
                AppRegistryEntryRecord::STATE_CONTRACT_NAME,
                AppRegistryData::STATE_CONTRACT_NAME,
            ),
        ] {
            let descriptor = descriptors
                .iter()
                .find(|descriptor| descriptor.allocation == allocation)
                .expect("topology registry descriptor");
            let declaration = descriptor
                .state
                .iter()
                .find(|declaration| declaration.domain == domain)
                .expect("topology registry state declaration");

            assert_eq!(declaration.record, record);
            assert_eq!(declaration.snapshot, snapshot);
        }
    }

    #[test]
    fn placement_descriptors_reference_canonical_data_types() {
        use crate::storage::stable::{
            directory::{DirectoryRegistryData, DirectoryRegistryEntryRecord},
            pool::{CanisterPoolData, CanisterPoolEntryRecord},
            scaling::{ScalingRegistryData, ScalingRegistryEntryRecord},
        };

        let descriptors = canic_state_descriptors();

        for (allocation, domain, record, snapshot) in [
            (
                StateAllocationKey::CanisterPool,
                "canister_pool",
                CanisterPoolEntryRecord::STATE_CONTRACT_NAME,
                CanisterPoolData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::ScalingRegistry,
                "scaling_registry",
                ScalingRegistryEntryRecord::STATE_CONTRACT_NAME,
                ScalingRegistryData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::DirectoryRegistry,
                "directory_registry",
                DirectoryRegistryEntryRecord::STATE_CONTRACT_NAME,
                DirectoryRegistryData::STATE_CONTRACT_NAME,
            ),
        ] {
            let descriptor = descriptors
                .iter()
                .find(|descriptor| descriptor.allocation == allocation)
                .expect("placement descriptor");
            let declaration = descriptor
                .state
                .iter()
                .find(|declaration| declaration.domain == domain)
                .expect("placement state declaration");

            assert_eq!(declaration.record, record);
            assert_eq!(declaration.snapshot, snapshot);
        }
    }

    #[test]
    fn sharding_descriptors_reference_canonical_data_types() {
        use crate::storage::stable::sharding::{
            ShardEntryRecord, ShardingActiveSetData, ShardingActiveSetRecord,
            ShardingAssignmentRecord, ShardingAssignmentsData, ShardingRegistryData,
        };

        let descriptors = canic_state_descriptors();

        for (allocation, domain, record, snapshot) in [
            (
                StateAllocationKey::ShardingRegistry,
                "sharding_registry",
                ShardEntryRecord::STATE_CONTRACT_NAME,
                ShardingRegistryData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::ShardingAssignments,
                "sharding_assignments",
                ShardingAssignmentRecord::STATE_CONTRACT_NAME,
                ShardingAssignmentsData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::ShardingActiveSet,
                "sharding_active_set",
                ShardingActiveSetRecord::STATE_CONTRACT_NAME,
                ShardingActiveSetData::STATE_CONTRACT_NAME,
            ),
        ] {
            let descriptor = descriptors
                .iter()
                .find(|descriptor| descriptor.allocation == allocation)
                .expect("sharding descriptor");
            let declaration = descriptor
                .state
                .iter()
                .find(|declaration| declaration.domain == domain)
                .expect("sharding state declaration");

            assert_eq!(declaration.record, record);
            assert_eq!(declaration.snapshot, snapshot);
        }
    }

    #[test]
    fn blob_storage_descriptors_reference_canonical_data_types() {
        use crate::storage::stable::blob_storage::{
            BlobDeletionPendingData, BlobDeletionPendingRecord, BlobStorageBillingStateData,
            BlobStorageBillingStateRecord, StorageGatewayPrincipalRecord,
            StorageGatewayPrincipalsData, StoredBlobRecord, StoredBlobsData,
        };

        let descriptors = canic_state_descriptors();

        for (allocation, domain, record, snapshot) in [
            (
                StateAllocationKey::StoredBlobs,
                "stored_blobs",
                StoredBlobRecord::STATE_CONTRACT_NAME,
                StoredBlobsData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::BlobDeletionPending,
                "blob_deletion_pending",
                BlobDeletionPendingRecord::STATE_CONTRACT_NAME,
                BlobDeletionPendingData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::StorageGatewayPrincipals,
                "storage_gateway_principals",
                StorageGatewayPrincipalRecord::STATE_CONTRACT_NAME,
                StorageGatewayPrincipalsData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::BlobStorageBilling,
                "blob_storage_billing",
                BlobStorageBillingStateRecord::STATE_CONTRACT_NAME,
                BlobStorageBillingStateData::STATE_CONTRACT_NAME,
            ),
        ] {
            let descriptor = descriptors
                .iter()
                .find(|descriptor| descriptor.allocation == allocation)
                .expect("blob-storage descriptor");
            let declaration = descriptor
                .state
                .iter()
                .find(|declaration| declaration.domain == domain)
                .expect("blob-storage state declaration");

            assert_eq!(declaration.record, record);
            assert_eq!(declaration.snapshot, snapshot);
        }
    }

    #[test]
    fn descriptors_track_reserved_core_memory_ids() {
        let descriptors = canic_state_descriptors();
        let ids = descriptors
            .iter()
            .flat_map(|descriptor| descriptor.reserved_memory.iter())
            .map(|reservation| reservation.memory_id)
            .collect::<Vec<_>>();

        for expected in [CYCLE_TRACKER_ID, LOG_INDEX_ID, LOG_DATA_ID] {
            assert!(
                ids.contains(&expected),
                "state manifest should reserve memory id {expected}"
            );
        }
    }
}
