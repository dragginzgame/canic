//! Stable records for one Fleet Subnet Root's exclusive physical-Canister inventory.

use canic_core::{
    cdk::{
        structures::{
            DefaultMemoryImpl, btreemap::BTreeMap as StableBtreeMap, cell::Cell,
            memory::VirtualMemory,
        },
        types::{Cycles, Principal},
    },
    control_plane_support::model::replay::ReplayCostGuardSettlement,
    eager_static,
    ids::{ComponentInstanceId, ReleaseBuildId},
    impl_storable_bounded, impl_storable_unbounded,
    role_contract::allocation::memory::control_plane::{
        ROOT_CANISTER_INVENTORY_ASSETS_ID, ROOT_CANISTER_POOL_HANDOFF_RECEIPTS_ID,
        ROOT_CANISTER_POOL_STATE_ID,
    },
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

eager_static! {
    static CANISTER_POOL: RefCell<
        StableBtreeMap<Principal, CanisterPoolAssetRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(StableBtreeMap::init(canic_core::ic_memory_key!(
        authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
        key = "canic.control_plane.root.canister_inventory.assets.v1",
        ty = CanisterPoolAssetRecord,
        id = ROOT_CANISTER_INVENTORY_ASSETS_ID
    )));
}

eager_static! {
    static CANISTER_POOL_HANDOFF_RECEIPTS: RefCell<
        StableBtreeMap<Principal, CanisterPoolHandoffReceiptRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(StableBtreeMap::init(canic_core::ic_memory_key!(
        authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
        key = "canic.control_plane.root.canister_pool.handoff_receipts.v1",
        ty = CanisterPoolHandoffReceiptRecord,
        id = ROOT_CANISTER_POOL_HANDOFF_RECEIPTS_ID
    )));
}

eager_static! {
    static CANISTER_POOL_STATE: RefCell<
        Cell<CanisterPoolStateRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(Cell::init(
        canic_core::ic_memory_key!(
            authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
            key = "canic.control_plane.root.canister_pool.state.v1",
            ty = CanisterPoolStateRecord,
            id = ROOT_CANISTER_POOL_STATE_ID
        ),
        CanisterPoolStateRecord::default(),
    ));
}

/// Why one autonomous pool refill stopped without a recoverable principal.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolCreationFailureRecord {
    UnresolvedAfterLedgerWindow,
    LedgerCreationFailed,
    LedgerRejected,
}

/// Durable progress of one exact Cycles Ledger pool-refill request.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolCreationProgressRecord {
    Intent {
        uncertain_result: bool,
    },
    Created {
        block_index: u64,
        canister_id: Principal,
    },
    Blocked {
        failure: CanisterPoolCreationFailureRecord,
    },
}

/// Exact authority frozen before one autonomous Cycles Ledger creation effect.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolCreationRecord {
    pub operation_id: [u8; 32],
    pub cycles_ledger: Principal,
    pub placement_subnet: Principal,
    pub root: Principal,
    pub ledger_amount: Cycles,
    pub created_at_time_ns: u64,
    pub prepared_at_ns: u64,
    pub cost_guard_settlement: Option<ReplayCostGuardSettlement>,
    pub progress: CanisterPoolCreationProgressRecord,
}

/// Singleton state required to recover exact pool refill and draining handoff effects.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolStateRecord {
    pub next_creation_sequence: u64,
    pub last_creation_timestamp_ns: u64,
    pub creation: Option<CanisterPoolCreationRecord>,
    pub handoff: Option<CanisterPoolHandoffRecord>,
    pub ledger_recovery: Option<CanisterPoolLedgerRecoveryRecord>,
    pub last_ledger_recovery: Option<CanisterPoolLedgerRecoveryReceiptRecord>,
}

impl CanisterPoolStateRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "CanisterPoolStateRecord";
}

impl_storable_bounded!(CanisterPoolStateRecord, 4_096, false);

/// Release-bound helper evidence retained by one pool Ledger recovery.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolLedgerRecoveryArtifactRecord {
    pub candid_sha256: [u8; 32],
    pub payload_hash: [u8; 32],
    pub payload_size_bytes: u64,
    pub raw_module_hash: [u8; 32],
    pub release_build_id: ReleaseBuildId,
}

/// Complete immutable authority of one pool Ledger recovery.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolLedgerRecoveryAuthorityRecord {
    pub artifact: CanisterPoolLedgerRecoveryArtifactRecord,
    pub canister_id: Principal,
    pub created_at_time_ns: u64,
    pub cycles_ledger: Principal,
    pub ledger_balance: Cycles,
    pub ledger_fee: Cycles,
    pub maximum_execution_burn_cycles: Cycles,
    pub operation_id: [u8; 32],
    pub withdrawal_amount: Cycles,
}

/// Durable progress of a Root-owned empty-pool Ledger recovery.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolLedgerRecoveryPhaseRecord {
    Prepared,
    HelperInstallIssued,
    HelperInstalled,
    WithdrawalIssued,
    WithdrawalVerified { block_index: u64 },
    HelperUninstallIssued { block_index: u64 },
}

/// Nonterminal current pool Ledger recovery.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolLedgerRecoveryRecord {
    pub authority: CanisterPoolLedgerRecoveryAuthorityRecord,
    pub initial_native_cycles: Cycles,
    pub phase: CanisterPoolLedgerRecoveryPhaseRecord,
    pub prepared_at_ns: u64,
}

/// Terminal receipt retained for exact at-most-once replay.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolLedgerRecoveryReceiptRecord {
    pub authority: CanisterPoolLedgerRecoveryAuthorityRecord,
    pub block_index: u64,
    pub completed_at_ns: u64,
    pub final_native_cycles: Cycles,
    pub initial_native_cycles: Cycles,
}

/// Exact retry authority for one draining-root asset handoff.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolHandoffRecord {
    pub canister_id: Principal,
    pub recipient: Principal,
    pub prepared_at_ns: u64,
}

/// Durable terminal receipt retaining exact replay after an asset leaves root inventory.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolHandoffReceiptRecord {
    pub recipient: Principal,
    pub completed_at_ns: u64,
}

impl CanisterPoolHandoffReceiptRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "CanisterPoolHandoffReceiptRecord";
}

impl_storable_bounded!(CanisterPoolHandoffReceiptRecord, 64, false);

/// Canonical snapshot identity for terminal pool handoff receipts.
pub struct CanisterPoolHandoffReceiptData;

impl CanisterPoolHandoffReceiptData {
    pub const STATE_CONTRACT_NAME: &'static str = "CanisterPoolHandoffReceiptData";
}

/// Durable identity of one Component allocation claiming an empty Canister.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolClaimRecord {
    pub component: ComponentInstanceId,
    pub operation_id: [u8; 32],
}

/// Durable reset outcome while a stopped workload is still Registry-owned.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolRecycleResetRecord {
    Pending,
    Ready,
    Failed(String),
}

/// Durable provenance for one physical Canister asset.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolAssetOriginRecord {
    InfrastructureStore,
    Created,
    Imported,
    Recycled,
}

/// Durable lifecycle state for one physical Canister asset.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolAssetStatusRecord {
    Store,
    StoreDeletionPending {
        operation_id: [u8; 32],
    },
    PendingReset,
    Ready,
    Claimed(CanisterPoolClaimRecord),
    Workload(CanisterPoolClaimRecord),
    Recycling {
        claim: CanisterPoolClaimRecord,
        reset: CanisterPoolRecycleResetRecord,
    },
    RecoveringLedger {
        operation_id: [u8; 32],
    },
    HandingOff {
        recipient: Principal,
    },
    Failed(String),
}

/// Complete persisted row for one physical Canister asset.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolAssetRecord {
    pub cycles: Cycles,
    pub origin: CanisterPoolAssetOriginRecord,
    pub status: CanisterPoolAssetStatusRecord,
    pub last_recycle: Option<CanisterPoolClaimRecord>,
    pub added_at_ns: u64,
    pub updated_at_ns: u64,
}

impl CanisterPoolAssetRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "CanisterPoolAssetRecord";
}

impl_storable_unbounded!(CanisterPoolAssetRecord);

/// One canonical export row including the stable-map key.
#[derive(Clone, Debug)]
pub struct CanisterPoolEntryRecord {
    pub canister_id: Principal,
    pub asset: CanisterPoolAssetRecord,
}

/// Canonical stable snapshot of the root-owned physical inventory.
#[derive(Clone, Debug)]
pub struct CanisterPoolData {
    pub entries: Vec<CanisterPoolEntryRecord>,
    pub state: CanisterPoolStateRecord,
}

impl CanisterPoolData {
    pub const STATE_CONTRACT_NAME: &'static str = "CanisterPoolData";
}

/// Stable-memory owner for the root-owned physical Canister inventory.
pub struct CanisterPoolStore;

impl CanisterPoolStore {
    #[must_use]
    pub fn get(canister_id: &Principal) -> Option<CanisterPoolAssetRecord> {
        CANISTER_POOL.with_borrow(|pool| pool.get(canister_id))
    }

    pub fn insert(
        canister_id: Principal,
        asset: CanisterPoolAssetRecord,
    ) -> Option<CanisterPoolAssetRecord> {
        CANISTER_POOL.with_borrow_mut(|pool| pool.insert(canister_id, asset))
    }

    pub fn remove(canister_id: &Principal) -> Option<CanisterPoolAssetRecord> {
        CANISTER_POOL.with_borrow_mut(|pool| pool.remove(canister_id))
    }

    #[must_use]
    pub fn export() -> CanisterPoolData {
        CanisterPoolData {
            entries: CANISTER_POOL.with_borrow(|pool| {
                pool.iter()
                    .map(|entry| CanisterPoolEntryRecord {
                        canister_id: *entry.key(),
                        asset: entry.value(),
                    })
                    .collect()
            }),
            state: CANISTER_POOL_STATE.with_borrow(|state| state.get().clone()),
        }
    }

    #[must_use]
    pub fn state() -> CanisterPoolStateRecord {
        CANISTER_POOL_STATE.with_borrow(|state| state.get().clone())
    }

    pub fn set_state(state: CanisterPoolStateRecord) {
        CANISTER_POOL_STATE.with_borrow_mut(|current| {
            current.set(state);
        });
    }

    #[must_use]
    pub fn handoff_receipt(canister_id: &Principal) -> Option<CanisterPoolHandoffReceiptRecord> {
        CANISTER_POOL_HANDOFF_RECEIPTS.with_borrow(|receipts| receipts.get(canister_id))
    }

    pub fn insert_handoff_receipt(
        canister_id: Principal,
        receipt: CanisterPoolHandoffReceiptRecord,
    ) -> Option<CanisterPoolHandoffReceiptRecord> {
        CANISTER_POOL_HANDOFF_RECEIPTS
            .with_borrow_mut(|receipts| receipts.insert(canister_id, receipt))
    }

    #[must_use]
    pub fn handoff_receipt_count() -> u64 {
        CANISTER_POOL_HANDOFF_RECEIPTS.with_borrow(StableBtreeMap::len)
    }

    #[cfg(test)]
    pub fn clear() {
        CANISTER_POOL.with_borrow_mut(StableBtreeMap::clear_new);
        CANISTER_POOL_HANDOFF_RECEIPTS.with_borrow_mut(StableBtreeMap::clear_new);
        Self::set_state(CanisterPoolStateRecord::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::{
        cdk::structures::{Storable, storable::Bound},
        ids::IntentId,
    };

    #[test]
    fn q6_current_asset_record_encodings_are_measured() {
        let asset_measurements = finite_asset_statuses()
            .into_iter()
            .map(|(name, status)| (name, maximum_asset(status).to_bytes().len()))
            .collect::<Vec<_>>();

        assert_eq!(
            asset_measurements,
            vec![
                ("store", 268),
                ("store_deletion_pending", 364),
                ("pending_reset", 275),
                ("ready", 268),
                ("claimed", 427),
                ("workload", 428),
                ("recycling_pending", 450),
                ("recycling_ready", 448),
                ("recycling_failed_empty", 451),
                ("handing_off", 316),
                ("recovering_ledger", 360),
                ("failed_empty", 271),
            ]
        );
    }

    #[test]
    fn q6_current_pool_state_and_receipt_encodings_are_measured() {
        let state_measurements = [
            (
                "default",
                CanisterPoolStateRecord::default().to_bytes().len(),
            ),
            (
                "intent",
                maximum_state(CanisterPoolCreationProgressRecord::Intent {
                    uncertain_result: true,
                })
                .to_bytes()
                .len(),
            ),
            (
                "created",
                maximum_state(CanisterPoolCreationProgressRecord::Created {
                    block_index: u64::MAX,
                    canister_id: maximum_principal(),
                })
                .to_bytes()
                .len(),
            ),
            (
                "blocked",
                maximum_state(CanisterPoolCreationProgressRecord::Blocked {
                    failure: CanisterPoolCreationFailureRecord::UnresolvedAfterLedgerWindow,
                })
                .to_bytes()
                .len(),
            ),
        ];
        assert_eq!(
            state_measurements,
            [
                ("default", 112),
                ("intent", 2_312),
                ("created", 2_359),
                ("blocked", 2_332),
            ]
        );
        assert!(state_measurements.iter().all(|(_, bytes)| *bytes <= 4_096));

        let handoff_receipt = CanisterPoolHandoffReceiptRecord {
            recipient: canister_principal(),
            completed_at_ns: u64::MAX,
        };
        assert_eq!(maximum_principal().to_bytes().len(), 29);
        assert_eq!(canister_principal().to_bytes().len(), 10);
        assert_eq!(handoff_receipt.to_bytes().len(), 47);
        assert!(handoff_receipt.to_bytes().len() <= 64);
    }

    #[test]
    fn q6_handoff_receipt_bound_does_not_cover_every_principal_value() {
        let structurally_maximal = CanisterPoolHandoffReceiptRecord {
            recipient: maximum_principal(),
            completed_at_ns: u64::MAX,
        };

        assert_eq!(structurally_maximal.to_bytes().len(), 67);
        assert!(structurally_maximal.to_bytes().len() > 64);
        assert!(matches!(
            CanisterPoolHandoffReceiptRecord::BOUND,
            Bound::Bounded {
                max_size: 64,
                is_fixed_size: false,
            }
        ));
    }

    #[test]
    fn q6_failure_text_keeps_asset_encoding_structurally_unbounded() {
        let failed_empty = maximum_asset(CanisterPoolAssetStatusRecord::Failed(String::new()))
            .to_bytes()
            .len();
        let failed_1_kib = maximum_asset(CanisterPoolAssetStatusRecord::Failed("x".repeat(1_024)))
            .to_bytes()
            .len();
        let recycling_empty = maximum_asset(CanisterPoolAssetStatusRecord::Recycling {
            claim: maximum_claim(),
            reset: CanisterPoolRecycleResetRecord::Failed(String::new()),
        })
        .to_bytes()
        .len();
        let recycling_1_kib = maximum_asset(CanisterPoolAssetStatusRecord::Recycling {
            claim: maximum_claim(),
            reset: CanisterPoolRecycleResetRecord::Failed("x".repeat(1_024)),
        })
        .to_bytes()
        .len();

        assert_eq!((failed_empty, failed_1_kib), (271, 1_297));
        assert_eq!((recycling_empty, recycling_1_kib), (451, 1_477));
        assert!(matches!(CanisterPoolAssetRecord::BOUND, Bound::Unbounded));
    }

    fn maximum_principal() -> Principal {
        Principal::from_slice(&[u8::MAX; 29])
    }

    fn canister_principal() -> Principal {
        Principal::from_slice(&[u8::MAX; 10])
    }

    fn finite_asset_statuses() -> [(&'static str, CanisterPoolAssetStatusRecord); 12] {
        let claim = maximum_claim();
        [
            ("store", CanisterPoolAssetStatusRecord::Store),
            (
                "store_deletion_pending",
                CanisterPoolAssetStatusRecord::StoreDeletionPending {
                    operation_id: [u8::MAX; 32],
                },
            ),
            ("pending_reset", CanisterPoolAssetStatusRecord::PendingReset),
            ("ready", CanisterPoolAssetStatusRecord::Ready),
            (
                "claimed",
                CanisterPoolAssetStatusRecord::Claimed(claim.clone()),
            ),
            (
                "workload",
                CanisterPoolAssetStatusRecord::Workload(claim.clone()),
            ),
            (
                "recycling_pending",
                CanisterPoolAssetStatusRecord::Recycling {
                    claim: claim.clone(),
                    reset: CanisterPoolRecycleResetRecord::Pending,
                },
            ),
            (
                "recycling_ready",
                CanisterPoolAssetStatusRecord::Recycling {
                    claim: claim.clone(),
                    reset: CanisterPoolRecycleResetRecord::Ready,
                },
            ),
            (
                "recycling_failed_empty",
                CanisterPoolAssetStatusRecord::Recycling {
                    claim,
                    reset: CanisterPoolRecycleResetRecord::Failed(String::new()),
                },
            ),
            (
                "handing_off",
                CanisterPoolAssetStatusRecord::HandingOff {
                    recipient: maximum_principal(),
                },
            ),
            (
                "recovering_ledger",
                CanisterPoolAssetStatusRecord::RecoveringLedger {
                    operation_id: [u8::MAX; 32],
                },
            ),
            (
                "failed_empty",
                CanisterPoolAssetStatusRecord::Failed(String::new()),
            ),
        ]
    }

    fn maximum_claim() -> CanisterPoolClaimRecord {
        CanisterPoolClaimRecord {
            component: ComponentInstanceId::from_generated_bytes([u8::MAX; 32]),
            operation_id: [u8::MAX; 32],
        }
    }

    fn maximum_asset(status: CanisterPoolAssetStatusRecord) -> CanisterPoolAssetRecord {
        CanisterPoolAssetRecord {
            cycles: Cycles::new(u128::MAX),
            origin: CanisterPoolAssetOriginRecord::Recycled,
            status,
            last_recycle: Some(maximum_claim()),
            added_at_ns: u64::MAX,
            updated_at_ns: u64::MAX,
        }
    }

    fn maximum_state(progress: CanisterPoolCreationProgressRecord) -> CanisterPoolStateRecord {
        CanisterPoolStateRecord {
            next_creation_sequence: u64::MAX,
            last_creation_timestamp_ns: u64::MAX,
            creation: Some(CanisterPoolCreationRecord {
                operation_id: [u8::MAX; 32],
                cycles_ledger: maximum_principal(),
                placement_subnet: maximum_principal(),
                root: maximum_principal(),
                ledger_amount: Cycles::new(u128::MAX),
                created_at_time_ns: u64::MAX,
                prepared_at_ns: u64::MAX,
                cost_guard_settlement: Some(ReplayCostGuardSettlement {
                    quota_intent_id: IntentId(u64::MAX),
                    reservation_intent_id: IntentId(u64::MAX),
                }),
                progress,
            }),
            handoff: Some(CanisterPoolHandoffRecord {
                canister_id: maximum_principal(),
                recipient: maximum_principal(),
                prepared_at_ns: u64::MAX,
            }),
            ledger_recovery: Some(CanisterPoolLedgerRecoveryRecord {
                authority: maximum_ledger_recovery_authority(),
                initial_native_cycles: Cycles::new(u128::MAX),
                phase: CanisterPoolLedgerRecoveryPhaseRecord::HelperUninstallIssued {
                    block_index: u64::MAX,
                },
                prepared_at_ns: u64::MAX,
            }),
            last_ledger_recovery: Some(CanisterPoolLedgerRecoveryReceiptRecord {
                authority: maximum_ledger_recovery_authority(),
                block_index: u64::MAX,
                completed_at_ns: u64::MAX,
                final_native_cycles: Cycles::new(u128::MAX),
                initial_native_cycles: Cycles::new(u128::MAX),
            }),
        }
    }

    fn maximum_ledger_recovery_authority() -> CanisterPoolLedgerRecoveryAuthorityRecord {
        CanisterPoolLedgerRecoveryAuthorityRecord {
            artifact: CanisterPoolLedgerRecoveryArtifactRecord {
                candid_sha256: [u8::MAX; 32],
                payload_hash: [u8::MAX; 32],
                payload_size_bytes: u64::MAX,
                raw_module_hash: [u8::MAX; 32],
                release_build_id: "ff".repeat(32).parse().expect("maximum release build ID"),
            },
            canister_id: maximum_principal(),
            created_at_time_ns: u64::MAX,
            cycles_ledger: maximum_principal(),
            ledger_balance: Cycles::new(u128::MAX),
            ledger_fee: Cycles::new(u128::MAX),
            maximum_execution_burn_cycles: Cycles::new(u128::MAX),
            operation_id: [u8::MAX; 32],
            withdrawal_amount: Cycles::new(u128::MAX),
        }
    }
}
