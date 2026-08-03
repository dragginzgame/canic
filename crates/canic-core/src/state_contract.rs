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
    application_receipt::{APPLICATION_RECEIPT_ELIGIBILITY_ID, APPLICATION_RECEIPT_REPLAY_ID},
    auth::AUTH_STATE_ID,
    authority_restore::AUTHORITY_RESTORE_FENCE_ID,
    blob_storage::{
        BLOB_STORAGE_BILLING_ID, BLOB_STORAGE_GATEWAY_PRINCIPALS_ID,
        BLOB_STORAGE_PENDING_DELETIONS_ID, BLOB_STORAGE_ROOTS_ID,
    },
    cycles::{
        CYCLES_FUNDING_LEDGER_ID, CYCLES_ICP_REFILL_RECORDS_ID, CYCLES_TOPUP_EVENTS_ID,
        CYCLES_TRACKER_ID,
    },
    fleet::{FLEET_ACTIVATION_ID, FLEET_STATE_ID},
    intent::{
        INTENT_EXPIRY_INDEX_ID, INTENT_META_ID, INTENT_PENDING_ID,
        INTENT_RECEIPT_BACKED_RECORDS_ID, INTENT_RECORDS_ID, INTENT_TOTALS_ID,
    },
    log::LOG_ENTRIES_ID,
    placement::{
        PLACEMENT_ACKNOWLEDGEMENT_INDEX_ID, PLACEMENT_INDEX_REGISTRY_ID,
        PLACEMENT_SCALING_REGISTRY_ID,
    },
    replay::REPLAY_RECEIPTS_ID,
    runtime::{RUNTIME_BINDINGS_ID, RUNTIME_CANISTER_CHILDREN_ID},
    sharding::{SHARDING_ACTIVE_SET_ID, SHARDING_ASSIGNMENTS_ID, SHARDING_REGISTRY_ID},
};
use crate::role_contract::{AllocationOwner, StateAllocationKey};

pub const STATE_MANIFEST_SCHEMA_VERSION: u16 = 1;

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
            StateAllocationKey::CoreRuntimeChildren,
            runtime_children_domains(),
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::CoreRuntimeBindings,
            runtime_bindings_domains(),
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::CoreFleetState,
            fleet_state_domains(),
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::CoreFleetActivation,
            fleet_activation_domains(),
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
        descriptor(StateAllocationKey::CoreCycles, cycles_domains(), Vec::new()),
        descriptor(
            StateAllocationKey::CoreCyclesIcpRefillRecords,
            icp_refill_domains(),
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::CoreRuntimeLog,
            runtime_log_domains(),
            Vec::new(),
        ),
        descriptor(StateAllocationKey::CoreIntent, intent_domains(), Vec::new()),
        descriptor(
            StateAllocationKey::CoreApplicationReceipts,
            application_receipt_domains(),
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::CorePlacementAcknowledgement,
            placement_acknowledgement_domains(),
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::CoreAuthorityRestoreFence,
            authority_restore_fence_domains(),
            Vec::new(),
        ),
    ]
}

fn placement_capacity_descriptors() -> Vec<StateAllocationDescriptor> {
    use crate::storage::stable::{
        placement_index::{PlacementIndexRegistryData, PlacementIndexRegistryEntryRecord},
        scaling::{ScalingRegistryData, ScalingRegistryEntryRecord},
    };

    vec![
        descriptor(
            StateAllocationKey::PlacementScalingRegistry,
            vec![state_domain(
                "placement_scaling_registry",
                PLACEMENT_SCALING_REGISTRY_ID,
                ScalingRegistryEntryRecord::STATE_CONTRACT_NAME,
                ScalingRegistryData::STATE_CONTRACT_NAME,
                140,
                "placement_scaling_registry_restores_worker_pool_membership",
            )],
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::PlacementIndexRegistry,
            vec![state_domain(
                "placement_index_registry",
                PLACEMENT_INDEX_REGISTRY_ID,
                PlacementIndexRegistryEntryRecord::STATE_CONTRACT_NAME,
                PlacementIndexRegistryData::STATE_CONTRACT_NAME,
                150,
                "placement_index_registry_entries_restore_entries",
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
                SHARDING_ASSIGNMENTS_ID,
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
            StateAllocationKey::BlobStorageRoots,
            vec![state_domain(
                "blob_storage_roots",
                BLOB_STORAGE_ROOTS_ID,
                StoredBlobRecord::STATE_CONTRACT_NAME,
                StoredBlobsData::STATE_CONTRACT_NAME,
                190,
                "blob_storage_roots_restore_live_blob_roots",
            )],
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::BlobStoragePendingDeletions,
            vec![state_domain(
                "blob_storage_pending_deletions",
                BLOB_STORAGE_PENDING_DELETIONS_ID,
                BlobDeletionPendingRecord::STATE_CONTRACT_NAME,
                BlobDeletionPendingData::STATE_CONTRACT_NAME,
                200,
                "blob_storage_pending_deletions_restore_gateway_scrub_state",
            )],
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::BlobStorageGatewayPrincipals,
            vec![state_domain(
                "blob_storage_gateway_principals",
                BLOB_STORAGE_GATEWAY_PRINCIPALS_ID,
                StorageGatewayPrincipalRecord::STATE_CONTRACT_NAME,
                StorageGatewayPrincipalsData::STATE_CONTRACT_NAME,
                210,
                "blob_storage_gateway_principals_restore_authorized_gateways",
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

fn runtime_children_domains() -> Vec<StateDomainManifest> {
    use crate::storage::{canister::CanisterEntryRecord, stable::children::CanisterChildrenData};

    vec![state_domain(
        "runtime_canister_children",
        RUNTIME_CANISTER_CHILDREN_ID,
        CanisterEntryRecord::STATE_CONTRACT_NAME,
        CanisterChildrenData::STATE_CONTRACT_NAME,
        30,
        "canister_children_projection_is_imported",
    )]
}

fn runtime_bindings_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::env::{EnvData, EnvRecord};

    vec![state_domain(
        "runtime_bindings",
        RUNTIME_BINDINGS_ID,
        EnvRecord::STATE_CONTRACT_NAME,
        EnvData::STATE_CONTRACT_NAME,
        40,
        "runtime_root_role_and_placement_bindings_are_restored",
    )]
}

fn fleet_state_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::state::fleet::{FleetStateData, FleetStateRecord};

    vec![state_domain(
        "fleet_state",
        FLEET_STATE_ID,
        FleetStateRecord::STATE_CONTRACT_NAME,
        FleetStateData::STATE_CONTRACT_NAME,
        50,
        "fleet_state_mode_is_restored_before_hooks",
    )]
}

fn auth_state_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::auth::{AuthStateData, AuthStateRecord};

    vec![state_domain(
        "auth_state",
        AUTH_STATE_ID,
        AuthStateRecord::STATE_CONTRACT_NAME,
        AuthStateData::STATE_CONTRACT_NAME,
        60,
        "auth_state_delegated_proofs_are_chain_key_only",
    )]
}

fn replay_receipt_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::replay::{ReplayReceiptRecord, ReplayReceiptsData};

    vec![state_domain(
        "replay_receipts",
        REPLAY_RECEIPTS_ID,
        ReplayReceiptRecord::STATE_CONTRACT_NAME,
        ReplayReceiptsData::STATE_CONTRACT_NAME,
        70,
        "replay_receipts_reject_unsupported_schema_versions",
    )]
}

fn fleet_activation_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::fleet_activation::{FleetActivationData, FleetActivationRecord};

    vec![state_domain(
        "fleet_activation",
        FLEET_ACTIVATION_ID,
        FleetActivationRecord::STATE_CONTRACT_NAME,
        FleetActivationData::STATE_CONTRACT_NAME,
        55,
        "fleet_activation_identity_and_phase_are_protected",
    )]
}

fn authority_restore_fence_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::authority_restore::{
        AuthorityRestoreFenceData, AuthorityRestoreFenceRecord,
    };

    vec![state_domain(
        "authority_restore_fence",
        AUTHORITY_RESTORE_FENCE_ID,
        AuthorityRestoreFenceRecord::STATE_CONTRACT_NAME,
        AuthorityRestoreFenceData::STATE_CONTRACT_NAME,
        57,
        "authority_snapshot_restore_remains_mutation_fenced_until_live_history_is_proven",
    )]
}

fn cycles_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::cycles::{
        CycleTopupEventRecord, CycleTopupEventsData, CycleTrackerData, CycleTrackerEntryRecord,
        CyclesFundingLedgerData, CyclesFundingLedgerRecord,
    };
    vec![
        state_domain(
            "cycles_tracker",
            CYCLES_TRACKER_ID,
            CycleTrackerEntryRecord::STATE_CONTRACT_NAME,
            CycleTrackerData::STATE_CONTRACT_NAME,
            75,
            "cycle_tracker_restores_ordered_balance_samples",
        ),
        state_domain(
            "cycles_topup_events",
            CYCLES_TOPUP_EVENTS_ID,
            CycleTopupEventRecord::STATE_CONTRACT_NAME,
            CycleTopupEventsData::STATE_CONTRACT_NAME,
            80,
            "cycle_topup_events_decode_status_values",
        ),
        state_domain(
            "cycles_funding_ledger",
            CYCLES_FUNDING_LEDGER_ID,
            CyclesFundingLedgerRecord::STATE_CONTRACT_NAME,
            CyclesFundingLedgerData::STATE_CONTRACT_NAME,
            90,
            "cycles_funding_ledger_restores_child_budget_state",
        ),
    ]
}

fn runtime_log_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::log::{LogEntriesData, LogEntryRecord};

    vec![state_domain(
        "runtime_log",
        LOG_ENTRIES_ID,
        LogEntryRecord::STATE_CONTRACT_NAME,
        LogEntriesData::STATE_CONTRACT_NAME,
        85,
        "runtime_log_restores_exact_sequence_and_retention_order",
    )]
}

fn icp_refill_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::icp_refill::{IcpRefillRecord, IcpRefillRecordsData};

    vec![state_domain(
        "cycles_icp_refill_records",
        CYCLES_ICP_REFILL_RECORDS_ID,
        IcpRefillRecord::STATE_CONTRACT_NAME,
        IcpRefillRecordsData::STATE_CONTRACT_NAME,
        100,
        "icp_refill_records_decode_status_and_error_codes",
    )]
}

fn intent_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::intent::{
        IntentExpiryEntryRecord, IntentExpiryIndexData, IntentMetaData, IntentPendingData,
        IntentPendingEntryRecord, IntentRecord, IntentRecordsData, IntentResourceTotalsRecord,
        IntentStoreMetaRecord, IntentTotalsData, ReceiptBackedIntentRecord,
        ReceiptBackedIntentsData,
    };

    vec![
        state_domain(
            "intent_meta",
            INTENT_META_ID,
            IntentStoreMetaRecord::STATE_CONTRACT_NAME,
            IntentMetaData::STATE_CONTRACT_NAME,
            110,
            "intent_meta_restores_schema_version",
        ),
        state_domain(
            "intent_records",
            INTENT_RECORDS_ID,
            IntentRecord::STATE_CONTRACT_NAME,
            IntentRecordsData::STATE_CONTRACT_NAME,
            111,
            "intent_records_restore_state_transitions",
        ),
        state_domain(
            "intent_totals",
            INTENT_TOTALS_ID,
            IntentResourceTotalsRecord::STATE_CONTRACT_NAME,
            IntentTotalsData::STATE_CONTRACT_NAME,
            112,
            "intent_totals_restore_resource_accounting",
        ),
        state_domain(
            "intent_pending",
            INTENT_PENDING_ID,
            IntentPendingEntryRecord::STATE_CONTRACT_NAME,
            IntentPendingData::STATE_CONTRACT_NAME,
            113,
            "intent_pending_entries_restore_ttl_metadata",
        ),
        state_domain(
            "intent_receipt_backed_records",
            INTENT_RECEIPT_BACKED_RECORDS_ID,
            ReceiptBackedIntentRecord::STATE_CONTRACT_NAME,
            ReceiptBackedIntentsData::STATE_CONTRACT_NAME,
            114,
            "intent_receipt_backed_records_restore_terminal_evidence",
        ),
        state_domain(
            "intent_expiry_index",
            INTENT_EXPIRY_INDEX_ID,
            IntentExpiryEntryRecord::STATE_CONTRACT_NAME,
            IntentExpiryIndexData::STATE_CONTRACT_NAME,
            115,
            "intent_expiry_index_restores_exact_ordered_deadlines",
        ),
    ]
}

fn application_receipt_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::intent::{
        ApplicationReceiptEligibilityData, ApplicationReceiptEligibilityRecord,
        ApplicationReceiptReplayData, ApplicationReceiptReplayRecord,
    };

    vec![
        state_domain(
            "application_receipt_replay",
            APPLICATION_RECEIPT_REPLAY_ID,
            ApplicationReceiptReplayRecord::STATE_CONTRACT_NAME,
            ApplicationReceiptReplayData::STATE_CONTRACT_NAME,
            116,
            "application_receipt_replay_restores_exact_deadlines",
        ),
        state_domain(
            "application_receipt_eligibility",
            APPLICATION_RECEIPT_ELIGIBILITY_ID,
            ApplicationReceiptEligibilityRecord::STATE_CONTRACT_NAME,
            ApplicationReceiptEligibilityData::STATE_CONTRACT_NAME,
            117,
            "application_receipt_eligibility_restores_exact_terminal_deadlines",
        ),
    ]
}

fn placement_acknowledgement_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::intent::{
        PlacementAcknowledgementEntryRecord, PlacementAcknowledgementIndexData,
    };

    vec![state_domain(
        "placement_acknowledgement_index",
        PLACEMENT_ACKNOWLEDGEMENT_INDEX_ID,
        PlacementAcknowledgementEntryRecord::STATE_CONTRACT_NAME,
        PlacementAcknowledgementIndexData::STATE_CONTRACT_NAME,
        118,
        "placement_acknowledgement_index_restores_exact_terminal_operations",
    )]
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
    fn descriptors_exactly_cover_declared_core_memory_ids() {
        let descriptors = canic_state_descriptors();
        let mut descriptor_ids = descriptors
            .iter()
            .flat_map(|descriptor| descriptor.state.iter())
            .filter_map(|domain| domain.memory_id)
            .collect::<Vec<_>>();
        let mut allocation_ids = crate::role_contract::allocation::allocation_definitions()
            .iter()
            .filter(|definition| definition.owner == AllocationOwner::CanicCore)
            .flat_map(|definition| definition.memory_ids)
            .map(|memory_id| memory_id.get())
            .collect::<Vec<_>>();

        descriptor_ids.sort_unstable();
        allocation_ids.sort_unstable();

        assert!(
            descriptors
                .iter()
                .all(|descriptor| descriptor.reserved_memory.is_empty())
        );
        assert_eq!(descriptor_ids, allocation_ids);
    }

    #[test]
    fn topology_registry_descriptors_reference_canonical_data_types() {
        use crate::storage::{
            canister::CanisterEntryRecord, stable::children::CanisterChildrenData,
        };

        let descriptors = canic_state_descriptors();
        let descriptor = descriptors
            .iter()
            .find(|descriptor| descriptor.allocation == StateAllocationKey::CoreRuntimeChildren)
            .expect("topology registry descriptor");
        let declaration = descriptor
            .state
            .iter()
            .find(|declaration| declaration.domain == "runtime_canister_children")
            .expect("Canister children state declaration");

        assert_eq!(declaration.record, CanisterEntryRecord::STATE_CONTRACT_NAME);
        assert_eq!(
            declaration.snapshot,
            CanisterChildrenData::STATE_CONTRACT_NAME
        );
    }

    #[test]
    fn runtime_bindings_and_fleet_state_descriptors_reference_canonical_data_types() {
        use crate::storage::stable::{
            env::{EnvData, EnvRecord},
            state::fleet::{FleetStateData, FleetStateRecord},
        };

        let descriptors = canic_state_descriptors();
        for (allocation, domain, record, snapshot) in [
            (
                StateAllocationKey::CoreRuntimeBindings,
                "runtime_bindings",
                EnvRecord::STATE_CONTRACT_NAME,
                EnvData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CoreFleetState,
                "fleet_state",
                FleetStateRecord::STATE_CONTRACT_NAME,
                FleetStateData::STATE_CONTRACT_NAME,
            ),
        ] {
            let descriptor = descriptors
                .iter()
                .find(|descriptor| descriptor.allocation == allocation)
                .expect("runtime bindings or Fleet-state descriptor");
            let declaration = descriptor
                .state
                .iter()
                .find(|declaration| declaration.domain == domain)
                .expect("runtime bindings or Fleet-state declaration");

            assert_eq!(declaration.record, record);
            assert_eq!(declaration.snapshot, snapshot);
        }
    }

    #[test]
    fn auth_and_replay_descriptors_reference_canonical_data_types() {
        use crate::storage::stable::{
            auth::{AuthStateData, AuthStateRecord},
            replay::{ReplayReceiptRecord, ReplayReceiptsData},
        };

        let descriptors = canic_state_descriptors();

        for (allocation, domain, record, snapshot) in [
            (
                StateAllocationKey::CoreAuthState,
                "auth_state",
                AuthStateRecord::STATE_CONTRACT_NAME,
                AuthStateData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CoreReplayReceipts,
                "replay_receipts",
                ReplayReceiptRecord::STATE_CONTRACT_NAME,
                ReplayReceiptsData::STATE_CONTRACT_NAME,
            ),
        ] {
            let descriptor = descriptors
                .iter()
                .find(|descriptor| descriptor.allocation == allocation)
                .expect("auth/replay descriptor");
            let declaration = descriptor
                .state
                .iter()
                .find(|declaration| declaration.domain == domain)
                .expect("auth/replay state declaration");

            assert_eq!(declaration.record, record);
            assert_eq!(declaration.snapshot, snapshot);
        }
    }

    #[test]
    fn cycles_and_log_descriptors_reference_canonical_data_types() {
        use crate::storage::stable::{
            cycles::{
                CycleTopupEventRecord, CycleTopupEventsData, CycleTrackerData,
                CycleTrackerEntryRecord, CyclesFundingLedgerData, CyclesFundingLedgerRecord,
            },
            icp_refill::{IcpRefillRecord, IcpRefillRecordsData},
            log::{LogEntriesData, LogEntryRecord},
        };

        let descriptors = canic_state_descriptors();

        for (allocation, domain, record, snapshot) in [
            (
                StateAllocationKey::CoreCycles,
                "cycles_tracker",
                CycleTrackerEntryRecord::STATE_CONTRACT_NAME,
                CycleTrackerData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CoreCycles,
                "cycles_topup_events",
                CycleTopupEventRecord::STATE_CONTRACT_NAME,
                CycleTopupEventsData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CoreRuntimeLog,
                "runtime_log",
                LogEntryRecord::STATE_CONTRACT_NAME,
                LogEntriesData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CoreCycles,
                "cycles_funding_ledger",
                CyclesFundingLedgerRecord::STATE_CONTRACT_NAME,
                CyclesFundingLedgerData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CoreCyclesIcpRefillRecords,
                "cycles_icp_refill_records",
                IcpRefillRecord::STATE_CONTRACT_NAME,
                IcpRefillRecordsData::STATE_CONTRACT_NAME,
            ),
        ] {
            let descriptor = descriptors
                .iter()
                .find(|descriptor| descriptor.allocation == allocation)
                .expect("observability descriptor");
            let declaration = descriptor
                .state
                .iter()
                .find(|declaration| declaration.domain == domain)
                .expect("observability state declaration");

            assert_eq!(declaration.record, record);
            assert_eq!(declaration.snapshot, snapshot);
        }
    }

    #[test]
    fn intent_descriptors_reference_canonical_data_types() {
        use crate::storage::stable::intent::{
            ApplicationReceiptEligibilityData, ApplicationReceiptEligibilityRecord,
            ApplicationReceiptReplayData, ApplicationReceiptReplayRecord, IntentExpiryEntryRecord,
            IntentExpiryIndexData, IntentMetaData, IntentPendingData, IntentPendingEntryRecord,
            IntentRecord, IntentRecordsData, IntentResourceTotalsRecord, IntentStoreMetaRecord,
            IntentTotalsData, PlacementAcknowledgementEntryRecord,
            PlacementAcknowledgementIndexData, ReceiptBackedIntentRecord, ReceiptBackedIntentsData,
        };

        let descriptors = canic_state_descriptors();
        for (allocation, domain, record, snapshot) in [
            (
                StateAllocationKey::CoreIntent,
                "intent_meta",
                IntentStoreMetaRecord::STATE_CONTRACT_NAME,
                IntentMetaData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CoreIntent,
                "intent_records",
                IntentRecord::STATE_CONTRACT_NAME,
                IntentRecordsData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CoreIntent,
                "intent_totals",
                IntentResourceTotalsRecord::STATE_CONTRACT_NAME,
                IntentTotalsData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CoreIntent,
                "intent_pending",
                IntentPendingEntryRecord::STATE_CONTRACT_NAME,
                IntentPendingData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CoreIntent,
                "intent_receipt_backed_records",
                ReceiptBackedIntentRecord::STATE_CONTRACT_NAME,
                ReceiptBackedIntentsData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CoreIntent,
                "intent_expiry_index",
                IntentExpiryEntryRecord::STATE_CONTRACT_NAME,
                IntentExpiryIndexData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CorePlacementAcknowledgement,
                "placement_acknowledgement_index",
                PlacementAcknowledgementEntryRecord::STATE_CONTRACT_NAME,
                PlacementAcknowledgementIndexData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CoreApplicationReceipts,
                "application_receipt_replay",
                ApplicationReceiptReplayRecord::STATE_CONTRACT_NAME,
                ApplicationReceiptReplayData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CoreApplicationReceipts,
                "application_receipt_eligibility",
                ApplicationReceiptEligibilityRecord::STATE_CONTRACT_NAME,
                ApplicationReceiptEligibilityData::STATE_CONTRACT_NAME,
            ),
        ] {
            let descriptor = descriptors
                .iter()
                .find(|descriptor| descriptor.allocation == allocation)
                .expect("intent-related descriptor");
            let declaration = descriptor
                .state
                .iter()
                .find(|declaration| declaration.domain == domain)
                .expect("intent-related state declaration");

            assert_eq!(declaration.record, record);
            assert_eq!(declaration.snapshot, snapshot);
        }

        assert!(
            descriptors
                .iter()
                .filter(|descriptor| {
                    matches!(
                        descriptor.allocation,
                        StateAllocationKey::CoreIntent
                            | StateAllocationKey::CoreApplicationReceipts
                            | StateAllocationKey::CorePlacementAcknowledgement
                    )
                })
                .all(|descriptor| descriptor.reserved_memory.is_empty())
        );
    }

    #[test]
    fn placement_descriptors_reference_canonical_data_types() {
        use crate::storage::stable::{
            placement_index::{PlacementIndexRegistryData, PlacementIndexRegistryEntryRecord},
            scaling::{ScalingRegistryData, ScalingRegistryEntryRecord},
        };

        let descriptors = canic_state_descriptors();

        for (allocation, domain, record, snapshot) in [
            (
                StateAllocationKey::PlacementScalingRegistry,
                "placement_scaling_registry",
                ScalingRegistryEntryRecord::STATE_CONTRACT_NAME,
                ScalingRegistryData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::PlacementIndexRegistry,
                "placement_index_registry",
                PlacementIndexRegistryEntryRecord::STATE_CONTRACT_NAME,
                PlacementIndexRegistryData::STATE_CONTRACT_NAME,
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
                StateAllocationKey::BlobStorageRoots,
                "blob_storage_roots",
                StoredBlobRecord::STATE_CONTRACT_NAME,
                StoredBlobsData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::BlobStoragePendingDeletions,
                "blob_storage_pending_deletions",
                BlobDeletionPendingRecord::STATE_CONTRACT_NAME,
                BlobDeletionPendingData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::BlobStorageGatewayPrincipals,
                "blob_storage_gateway_principals",
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
}
