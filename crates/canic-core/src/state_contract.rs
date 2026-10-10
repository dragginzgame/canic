//! Module: state_contract
//!
//! Responsibility: declare Canic-owned stable state metadata for host-side
//! state manifest and audit reports.
//! Does not own: CLI rendering, stable-memory reads, or stable-memory writes.
//! Boundary: declarations are static Rust metadata derived from the storage
//! modules that own the records and memory keys.

use crate::role_contract::{
    AllocationOwner, StateAllocationKey,
    allocation::memory::{
        application_receipt::APPLICATION_RECEIPT_ELIGIBILITY_KEY,
        async_job_recovery::ASYNC_JOB_RECOVERY_KEY,
        auth::{
            DELEGATED_TOKEN_ISSUER_STATE_KEY, LOCAL_APPLICATION_AUTHORIZATION_STATE_KEY,
            ROOT_DELEGATION_STATE_KEY,
        },
        authority_restore::AUTHORITY_RESTORE_FENCE_KEY,
        cycles::{
            CYCLES_FUNDING_LEDGER_KEY, CYCLES_ICP_REFILL_RECORDS_KEY, CYCLES_TOPUP_EVENTS_KEY,
            CYCLES_TRACKER_KEY,
        },
        fleet::{FLEET_ACTIVATION_KEY, FLEET_STATE_KEY},
        fleet_admission_projection::FLEET_ADMISSION_PROJECTION_KEY,
        intent::{
            INTENT_EXPIRY_INDEX_KEY, INTENT_META_KEY, INTENT_PENDING_KEY,
            INTENT_RECEIPT_BACKED_RECORDS_KEY, INTENT_RECORDS_KEY, INTENT_TOTALS_KEY,
        },
        log::LOG_ENTRIES_KEY,
        placement::{
            PLACEMENT_ACKNOWLEDGEMENT_INDEX_KEY, PLACEMENT_INDEX_REGISTRY_KEY,
            PLACEMENT_SCALING_REGISTRY_KEY,
        },
        replay::REPLAY_RECEIPTS_KEY,
        runtime::{RUNTIME_BINDINGS_KEY, RUNTIME_CANISTER_CHILDREN_KEY},
        sharding::{SHARDING_ASSIGNMENTS_KEY, SHARDING_REGISTRY_KEY},
    },
};
use serde::Serialize;

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
    pub memory_key: Option<String>,
    pub owner: String,
    pub record: String,
    pub snapshot: String,
    pub restore_order: Option<u32>,
    pub post_upgrade_invariant: Option<String>,
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
/// ReservedMemoryManifest
///
/// Explicit reservation for a stable memory key whose persisted state shape is
/// known but not yet represented as one active state domain.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReservedMemoryManifest {
    pub label: String,
    pub memory_key: String,
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
            StateAllocationKey::CoreCallerAuthority,
            caller_authority_domains(),
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::CoreLocalApplicationAuthorizationState,
            local_application_authorization_state_domains(),
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::CoreDelegatedTokenIssuerState,
            delegated_token_issuer_state_domains(),
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::CoreRootDelegationState,
            root_delegation_state_domains(),
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
        descriptor(
            StateAllocationKey::CoreAsyncJobRecovery,
            async_job_recovery_domains(),
            Vec::new(),
        ),
        descriptor(
            StateAllocationKey::CoreFleetAdmissionProjection,
            fleet_admission_projection_domains(),
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
                PLACEMENT_SCALING_REGISTRY_KEY,
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
                PLACEMENT_INDEX_REGISTRY_KEY,
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
        ShardEntryRecord, ShardingAssignmentRecord, ShardingAssignmentsData, ShardingRegistryData,
    };

    vec![
        descriptor(
            StateAllocationKey::ShardingRegistry,
            vec![state_domain(
                "sharding_registry",
                SHARDING_REGISTRY_KEY,
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
                SHARDING_ASSIGNMENTS_KEY,
                ShardingAssignmentRecord::STATE_CONTRACT_NAME,
                ShardingAssignmentsData::STATE_CONTRACT_NAME,
                170,
                "sharding_assignments_restore_partition_bindings",
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
    reserved_memory.sort_by(|a, b| a.memory_key.cmp(&b.memory_key));
    StateAllocationDescriptor {
        allocation,
        owner: AllocationOwner::CanicCore,
        state,
        reserved_memory,
    }
}

fn runtime_children_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::children::{CanisterChildEntryRecord, CanisterChildrenData};

    vec![state_domain(
        "runtime_canister_children",
        RUNTIME_CANISTER_CHILDREN_KEY,
        CanisterChildEntryRecord::STATE_CONTRACT_NAME,
        CanisterChildrenData::STATE_CONTRACT_NAME,
        30,
        "canister_children_projection_is_imported",
    )]
}

fn runtime_bindings_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::env::{EnvData, EnvRecord};

    vec![state_domain(
        "runtime_bindings",
        RUNTIME_BINDINGS_KEY,
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
        FLEET_STATE_KEY,
        FleetStateRecord::STATE_CONTRACT_NAME,
        FleetStateData::STATE_CONTRACT_NAME,
        50,
        "fleet_state_mode_is_restored_before_hooks",
    )]
}

fn local_application_authorization_state_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::auth::{
        LocalApplicationAuthorizationStateData, LocalApplicationAuthorizationStateRecord,
    };

    vec![state_domain(
        "local_application_authorization",
        LOCAL_APPLICATION_AUTHORIZATION_STATE_KEY,
        LocalApplicationAuthorizationStateRecord::STATE_CONTRACT_NAME,
        LocalApplicationAuthorizationStateData::STATE_CONTRACT_NAME,
        60,
        "application_sessions_match_the_current_local_authority_generation",
    )]
}

fn delegated_token_issuer_state_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::auth::{
        DelegatedTokenIssuerStateData, DelegatedTokenIssuerStateRecord,
    };

    vec![state_domain(
        "delegated_token_issuer",
        DELEGATED_TOKEN_ISSUER_STATE_KEY,
        DelegatedTokenIssuerStateRecord::STATE_CONTRACT_NAME,
        DelegatedTokenIssuerStateData::STATE_CONTRACT_NAME,
        61,
        "active_delegation_proof_is_bound_to_the_current_issuer",
    )]
}

fn root_delegation_state_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::auth::{RootDelegationStateData, RootDelegationStateRecord};

    vec![state_domain(
        "root_delegation",
        ROOT_DELEGATION_STATE_KEY,
        RootDelegationStateRecord::STATE_CONTRACT_NAME,
        RootDelegationStateData::STATE_CONTRACT_NAME,
        62,
        "root_delegation_policy_epochs_and_batches_are_monotonic",
    )]
}

fn replay_receipt_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::replay::{ReplayReceiptRecord, ReplayReceiptsData};

    vec![state_domain(
        "replay_receipts",
        REPLAY_RECEIPTS_KEY,
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
        FLEET_ACTIVATION_KEY,
        FleetActivationRecord::STATE_CONTRACT_NAME,
        FleetActivationData::STATE_CONTRACT_NAME,
        55,
        "fleet_activation_identity_and_phase_are_protected",
    )]
}

fn caller_authority_domains() -> Vec<StateDomainManifest> {
    use crate::model::caller_authority::CallerReceiverRecord;
    use crate::role_contract::allocation::memory::caller_authority::{
        CALLER_AUTHORITY_HEADER_KEY, CALLER_AUTHORITY_ROWS_KEY,
    };
    use crate::storage::stable::caller_authority::{CallerAuthorityData, CallerRowRecord};
    vec![
        state_domain(
            "caller_authority_header",
            CALLER_AUTHORITY_HEADER_KEY,
            CallerReceiverRecord::STATE_CONTRACT_NAME,
            CallerAuthorityData::STATE_CONTRACT_NAME,
            63,
            "exact_installation_bound_caller_publication",
        ),
        state_domain(
            "caller_authority_rows",
            CALLER_AUTHORITY_ROWS_KEY,
            CallerRowRecord::STATE_CONTRACT_NAME,
            CallerAuthorityData::STATE_CONTRACT_NAME,
            64,
            "source_fences_and_original_operation_receipts",
        ),
    ]
}

fn authority_restore_fence_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::authority_restore::{
        AuthorityRestoreFenceData, AuthorityRestoreFenceRecord,
    };

    vec![state_domain(
        "authority_restore_fence",
        AUTHORITY_RESTORE_FENCE_KEY,
        AuthorityRestoreFenceRecord::STATE_CONTRACT_NAME,
        AuthorityRestoreFenceData::STATE_CONTRACT_NAME,
        57,
        "authority_snapshot_restore_remains_mutation_fenced_until_live_history_is_proven",
    )]
}

fn async_job_recovery_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::async_job_recovery::{
        AsyncJobRecoveryData, AsyncJobRecoveryRecord,
    };

    vec![state_domain(
        "async_job_recovery",
        ASYNC_JOB_RECOVERY_KEY,
        AsyncJobRecoveryRecord::STATE_CONTRACT_NAME,
        AsyncJobRecoveryData::STATE_CONTRACT_NAME,
        58,
        "async_job_recovery_restores_exact_serial_attempt_fences",
    )]
}

fn fleet_admission_projection_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::fleet_admission_projection::{
        FleetAdmissionProjectionData, FleetAdmissionProjectionRecord,
    };

    vec![state_domain(
        "fleet_admission_projection",
        FLEET_ADMISSION_PROJECTION_KEY,
        FleetAdmissionProjectionRecord::STATE_CONTRACT_NAME,
        FleetAdmissionProjectionData::STATE_CONTRACT_NAME,
        59,
        "fleet_admission_projection_restores_exact_target_authority_without_reseeding",
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
            CYCLES_TRACKER_KEY,
            CycleTrackerEntryRecord::STATE_CONTRACT_NAME,
            CycleTrackerData::STATE_CONTRACT_NAME,
            75,
            "cycle_tracker_restores_ordered_balance_samples",
        ),
        state_domain(
            "cycles_topup_events",
            CYCLES_TOPUP_EVENTS_KEY,
            CycleTopupEventRecord::STATE_CONTRACT_NAME,
            CycleTopupEventsData::STATE_CONTRACT_NAME,
            80,
            "cycle_topup_events_decode_status_values",
        ),
        state_domain(
            "cycles_funding_ledger",
            CYCLES_FUNDING_LEDGER_KEY,
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
        LOG_ENTRIES_KEY,
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
        CYCLES_ICP_REFILL_RECORDS_KEY,
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
            INTENT_META_KEY,
            IntentStoreMetaRecord::STATE_CONTRACT_NAME,
            IntentMetaData::STATE_CONTRACT_NAME,
            110,
            "intent_meta_restores_schema_version",
        ),
        state_domain(
            "intent_records",
            INTENT_RECORDS_KEY,
            IntentRecord::STATE_CONTRACT_NAME,
            IntentRecordsData::STATE_CONTRACT_NAME,
            111,
            "intent_records_restore_state_transitions",
        ),
        state_domain(
            "intent_totals",
            INTENT_TOTALS_KEY,
            IntentResourceTotalsRecord::STATE_CONTRACT_NAME,
            IntentTotalsData::STATE_CONTRACT_NAME,
            112,
            "intent_totals_restore_resource_accounting",
        ),
        state_domain(
            "intent_pending",
            INTENT_PENDING_KEY,
            IntentPendingEntryRecord::STATE_CONTRACT_NAME,
            IntentPendingData::STATE_CONTRACT_NAME,
            113,
            "intent_pending_entries_restore_ttl_metadata",
        ),
        state_domain(
            "intent_receipt_backed_records",
            INTENT_RECEIPT_BACKED_RECORDS_KEY,
            ReceiptBackedIntentRecord::STATE_CONTRACT_NAME,
            ReceiptBackedIntentsData::STATE_CONTRACT_NAME,
            114,
            "intent_receipt_backed_records_restore_terminal_evidence",
        ),
        state_domain(
            "intent_expiry_index",
            INTENT_EXPIRY_INDEX_KEY,
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
    };

    vec![state_domain(
        "application_receipt_eligibility",
        APPLICATION_RECEIPT_ELIGIBILITY_KEY,
        ApplicationReceiptEligibilityRecord::STATE_CONTRACT_NAME,
        ApplicationReceiptEligibilityData::STATE_CONTRACT_NAME,
        117,
        "application_receipt_eligibility_restores_exact_terminal_deadlines",
    )]
}

fn placement_acknowledgement_domains() -> Vec<StateDomainManifest> {
    use crate::storage::stable::intent::{
        PlacementAcknowledgementEntryRecord, PlacementAcknowledgementIndexData,
    };

    vec![state_domain(
        "placement_acknowledgement_index",
        PLACEMENT_ACKNOWLEDGEMENT_INDEX_KEY,
        PlacementAcknowledgementEntryRecord::STATE_CONTRACT_NAME,
        PlacementAcknowledgementIndexData::STATE_CONTRACT_NAME,
        118,
        "placement_acknowledgement_index_restores_exact_terminal_operations",
    )]
}

fn state_domain(
    domain: &str,
    memory_key: &str,
    record: &str,
    snapshot: &str,
    restore_order: u32,
    invariant: &str,
) -> StateDomainManifest {
    StateDomainManifest {
        domain: domain.to_string(),
        version: 1,
        storage: StateStorage::StableMemory,
        memory_key: Some(memory_key.to_string()),
        owner: AllocationOwner::CanicCore.as_str().to_string(),
        record: record.to_string(),
        snapshot: snapshot.to_string(),
        restore_order: Some(restore_order),
        post_upgrade_invariant: Some(invariant.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptors_use_unique_memory_keys() {
        let descriptors = canic_state_descriptors();
        let mut ids = descriptors
            .iter()
            .flat_map(|descriptor| {
                descriptor
                    .state
                    .iter()
                    .filter_map(|domain| domain.memory_key.clone())
                    .chain(
                        descriptor
                            .reserved_memory
                            .iter()
                            .map(|reservation| reservation.memory_key.clone()),
                    )
            })
            .collect::<Vec<_>>();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), count);
    }

    #[test]
    fn state_contract_storage_owns_serialized_labels() {
        assert_eq!(StateStorage::StableMemory.as_str(), "stable_memory");
        assert_eq!(StateStorage::HeapOnly.as_str(), "heap_only");
        assert_eq!(StateStorage::NotApplicable.as_str(), "not_applicable");
    }

    #[test]
    fn current_state_domains_use_exactly_schema_version_one() {
        assert!(
            canic_state_descriptors()
                .iter()
                .all(|descriptor| { descriptor.state.iter().all(|domain| domain.version == 1) })
        );
    }

    #[test]
    fn descriptors_exactly_cover_declared_core_memory_keys() {
        let descriptors = canic_state_descriptors();
        let mut descriptor_ids = descriptors
            .iter()
            .flat_map(|descriptor| descriptor.state.iter())
            .filter_map(|domain| domain.memory_key.clone())
            .collect::<Vec<_>>();
        let mut allocation_ids = crate::role_contract::allocation::allocation_definitions()
            .iter()
            .filter(|definition| definition.owner == AllocationOwner::CanicCore)
            .flat_map(|definition| definition.memory_keys)
            .map(ToString::to_string)
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
        use crate::storage::stable::children::{CanisterChildEntryRecord, CanisterChildrenData};

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

        assert_eq!(
            declaration.record,
            CanisterChildEntryRecord::STATE_CONTRACT_NAME
        );
        assert_eq!(
            declaration.snapshot,
            CanisterChildrenData::STATE_CONTRACT_NAME
        );
    }

    #[test]
    fn runtime_bindings_and_fleet_state_descriptors_reference_canonical_data_types() {
        use crate::storage::stable::{
            env::{EnvData, EnvRecord},
            fleet_admission_projection::{
                FleetAdmissionProjectionData, FleetAdmissionProjectionRecord,
            },
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
            (
                StateAllocationKey::CoreFleetAdmissionProjection,
                "fleet_admission_projection",
                FleetAdmissionProjectionRecord::STATE_CONTRACT_NAME,
                FleetAdmissionProjectionData::STATE_CONTRACT_NAME,
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
            auth::{
                DelegatedTokenIssuerStateData, DelegatedTokenIssuerStateRecord,
                LocalApplicationAuthorizationStateData, LocalApplicationAuthorizationStateRecord,
                RootDelegationStateData, RootDelegationStateRecord,
            },
            replay::{ReplayReceiptRecord, ReplayReceiptsData},
        };

        let descriptors = canic_state_descriptors();

        for (allocation, domain, record, snapshot) in [
            (
                StateAllocationKey::CoreLocalApplicationAuthorizationState,
                "local_application_authorization",
                LocalApplicationAuthorizationStateRecord::STATE_CONTRACT_NAME,
                LocalApplicationAuthorizationStateData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CoreDelegatedTokenIssuerState,
                "delegated_token_issuer",
                DelegatedTokenIssuerStateRecord::STATE_CONTRACT_NAME,
                DelegatedTokenIssuerStateData::STATE_CONTRACT_NAME,
            ),
            (
                StateAllocationKey::CoreRootDelegationState,
                "root_delegation",
                RootDelegationStateRecord::STATE_CONTRACT_NAME,
                RootDelegationStateData::STATE_CONTRACT_NAME,
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
            IntentExpiryEntryRecord, IntentExpiryIndexData, IntentMetaData, IntentPendingData,
            IntentPendingEntryRecord, IntentRecord, IntentRecordsData, IntentResourceTotalsRecord,
            IntentStoreMetaRecord, IntentTotalsData, PlacementAcknowledgementEntryRecord,
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
            ShardEntryRecord, ShardingAssignmentRecord, ShardingAssignmentsData,
            ShardingRegistryData,
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
}
