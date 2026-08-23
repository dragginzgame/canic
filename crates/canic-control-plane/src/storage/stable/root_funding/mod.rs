//! Module: storage::stable::root_funding
//!
//! Responsibility: own one Root's bounded Coordinator-funding request journal.
//! Does not own: Registry validation, caller authentication, balance reads, or cycle acceptance.
//! Boundary: Root funding ops commit only complete validated current or terminal records.

use canic_core::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    dto::fleet_funding::{
        FleetFundingPolicyRotationRootPrepareRequest, FleetFundingPolicyRotationRootReceipt,
        FleetRootFundingAcceptanceReceipt, FleetRootFundingRequest, FleetRootFundingResponse,
    },
    eager_static, impl_storable_bounded,
    role_contract::allocation::memory::control_plane::ROOT_FUNDING_ID,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

const ROOT_FUNDING_STATE_MAX_BYTES: u32 = 32_768;

struct RootFundingState;

eager_static! {
    static ROOT_FUNDING_STATE:
        RefCell<Cell<RootFundingStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            canic_core::ic_memory_key!(
                authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
                key = "canic.control_plane.root.funding.v1",
                ty = RootFundingState,
                id = ROOT_FUNDING_ID
            ),
            RootFundingStateRecord::default(),
        ));
}

/// Schema identity for the reinstall-only Root funding journal.
pub const ROOT_FUNDING_SCHEMA_VERSION: u16 = 1;

/// Nonterminal phase of the one current Root funding operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootFundingActivePhaseRecord {
    CoordinatorRequested,
    GrantAccepted(Box<FleetRootFundingAcceptanceReceipt>),
}

/// One durable operation retained before the outbound Coordinator call.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFundingActiveOperationRecord {
    pub request: FleetRootFundingRequest,
    pub phase: RootFundingActivePhaseRecord,
    pub opened_at_ns: u64,
    pub updated_at_ns: u64,
}

/// Exact last terminal result retained until its monotonic successor completes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFundingTerminalOperationRecord {
    pub request: FleetRootFundingRequest,
    pub response: FleetRootFundingResponse,
    pub opened_at_ns: u64,
    pub completed_at_ns: u64,
}

/// Complete bounded request and acceptance journal for one Root.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFundingRecord {
    pub schema_version: u16,
    pub policy_generation: u64,
    pub historical_automatic_grants: u64,
    pub historical_automatic_cycles: canic_core::cdk::types::Cycles,
    pub automatic_grants: u32,
    pub automatic_cycles: canic_core::cdk::types::Cycles,
    pub current: Option<RootFundingActiveOperationRecord>,
    pub last: Option<RootFundingTerminalOperationRecord>,
    pub rotation_current: Option<RootFundingPolicyRotationRecord>,
    pub rotation_last: Option<RootFundingPolicyRotationTerminalRecord>,
}

impl Default for RootFundingRecord {
    fn default() -> Self {
        Self {
            schema_version: ROOT_FUNDING_SCHEMA_VERSION,
            policy_generation: 1,
            historical_automatic_grants: 0,
            historical_automatic_cycles: canic_core::cdk::types::Cycles::new(0),
            automatic_grants: 0,
            automatic_cycles: canic_core::cdk::types::Cycles::new(0),
            current: None,
            last: None,
            rotation_current: None,
            rotation_last: None,
        }
    }
}

/// One Root-owned prepared policy rotation retained until activation converges.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFundingPolicyRotationRecord {
    pub request: FleetFundingPolicyRotationRootPrepareRequest,
    pub prepared_receipt: FleetFundingPolicyRotationRootReceipt,
}

/// Exact terminal request and receipt retained for lossless activation replay.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFundingPolicyRotationTerminalRecord {
    pub request: FleetFundingPolicyRotationRootPrepareRequest,
    pub receipt: FleetFundingPolicyRotationRootReceipt,
}

impl RootFundingRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "RootFundingRecord";
}

/// Stable optional wrapper used before fresh Root initialization.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootFundingStateRecord {
    pub current: Option<RootFundingRecord>,
}

impl_storable_bounded!(RootFundingStateRecord, ROOT_FUNDING_STATE_MAX_BYTES, false);

/// Canonical export snapshot for the Root funding allocation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RootFundingData {
    pub current: Option<RootFundingRecord>,
}

impl RootFundingData {
    pub const STATE_CONTRACT_NAME: &'static str = "RootFundingData";
}

/// Result of one exact stable journal commit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootFundingCommitOutcome {
    Committed,
    Existing,
}

/// Stable-store rejection when expected Root funding state is not current.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootFundingCommitError {
    ConflictingState,
    Uninitialized,
}

/// Narrow stable-storage owner used only by Root funding ops.
pub struct RootFundingStore;

impl RootFundingStore {
    pub(crate) fn commit_genesis(
        record: RootFundingRecord,
    ) -> Result<RootFundingCommitOutcome, RootFundingCommitError> {
        ROOT_FUNDING_STATE.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            match state.current.as_ref() {
                None => {
                    state.current = Some(record);
                    cell.set(state);
                    Ok(RootFundingCommitOutcome::Committed)
                }
                Some(existing) if existing == &record => Ok(RootFundingCommitOutcome::Existing),
                Some(_) => Err(RootFundingCommitError::ConflictingState),
            }
        })
    }

    pub(crate) fn commit_transition(
        expected: &RootFundingRecord,
        next: RootFundingRecord,
    ) -> Result<RootFundingCommitOutcome, RootFundingCommitError> {
        ROOT_FUNDING_STATE.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            match state.current.as_ref() {
                None => Err(RootFundingCommitError::Uninitialized),
                Some(existing) if existing == &next => Ok(RootFundingCommitOutcome::Existing),
                Some(existing) if existing != expected => {
                    Err(RootFundingCommitError::ConflictingState)
                }
                Some(_) => {
                    state.current = Some(next);
                    cell.set(state);
                    Ok(RootFundingCommitOutcome::Committed)
                }
            }
        })
    }

    #[must_use]
    pub(crate) fn export() -> RootFundingData {
        ROOT_FUNDING_STATE.with_borrow(|cell| RootFundingData {
            current: cell.get().current.clone(),
        })
    }

    #[cfg(test)]
    pub(crate) fn import(data: RootFundingData) {
        ROOT_FUNDING_STATE.with_borrow_mut(|cell| {
            cell.set(RootFundingStateRecord {
                current: data.current,
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::{
        cdk::{structures::storable::Storable, types::Cycles},
        dto::fleet_funding::{
            FleetFundingPolicyRotationPlacementEvidence, FleetFundingPolicyRotationRootPlan,
            FleetFundingPolicyUsage,
        },
        ids::SubnetId,
    };

    #[test]
    fn maximum_format_root_journal_fits_its_stable_bound() {
        let request = crate::test_support::root_funding_request_fixture(2);
        let receipt =
            crate::test_support::root_funding_acceptance_receipt_fixture(&request, u64::MAX);
        let rotation_request = FleetFundingPolicyRotationRootPrepareRequest {
            operation_id: [91; 32],
            plan_digest: [92; 32],
            predecessor_registry: request.expected_registry.clone(),
            predecessor_generation: u64::MAX - 1,
            successor_generation: u64::MAX,
            root: FleetFundingPolicyRotationRootPlan {
                fleet_subnet_root: receipt.fleet_subnet_root,
                predecessor_policy_hash: [93; 32],
                predecessor_usage: FleetFundingPolicyUsage {
                    historical_automatic_grants: u64::MAX,
                    historical_automatic_cycles: Cycles::new(u128::MAX),
                    generation_automatic_grants: u32::MAX,
                    generation_automatic_cycles: Cycles::new(u128::MAX),
                },
                proposed_policy: crate::test_support::fleet_subnet_root_funding_authority()
                    .root_funding,
                placement: FleetFundingPolicyRotationPlacementEvidence {
                    subnet: SubnetId::from_principal(candid::Principal::from_slice(&[94; 29])),
                    node_count: u64::MAX,
                    cost_multiplier_numerator: u64::MAX,
                    cost_multiplier_denominator: u64::MAX,
                    fiduciary: true,
                    acknowledge_fiduciary_cost: true,
                },
            },
        };
        let prepared_receipt = FleetFundingPolicyRotationRootReceipt {
            operation_id: rotation_request.operation_id,
            plan_digest: rotation_request.plan_digest,
            fleet_subnet_root: rotation_request.root.fleet_subnet_root,
            predecessor_generation: rotation_request.predecessor_generation,
            successor_generation: rotation_request.successor_generation,
            prepared: true,
            activated: false,
            recorded_at_ns: u64::MAX,
        };
        let mut terminal_receipt = prepared_receipt.clone();
        terminal_receipt.activated = true;
        let state = RootFundingStateRecord {
            current: Some(RootFundingRecord {
                schema_version: ROOT_FUNDING_SCHEMA_VERSION,
                policy_generation: u64::MAX,
                historical_automatic_grants: u64::MAX,
                historical_automatic_cycles: Cycles::new(u128::MAX),
                automatic_grants: u32::MAX,
                automatic_cycles: Cycles::new(u128::MAX),
                current: Some(RootFundingActiveOperationRecord {
                    request: request.clone(),
                    phase: RootFundingActivePhaseRecord::GrantAccepted(Box::new(receipt)),
                    opened_at_ns: u64::MAX,
                    updated_at_ns: u64::MAX,
                }),
                last: Some(RootFundingTerminalOperationRecord {
                    request: request.clone(),
                    response: FleetRootFundingResponse::Granted(
                        crate::test_support::root_funding_acceptance_receipt_fixture(
                            &request,
                            u64::MAX,
                        ),
                    ),
                    opened_at_ns: u64::MAX,
                    completed_at_ns: u64::MAX,
                }),
                rotation_current: Some(RootFundingPolicyRotationRecord {
                    request: rotation_request.clone(),
                    prepared_receipt,
                }),
                rotation_last: Some(RootFundingPolicyRotationTerminalRecord {
                    request: rotation_request,
                    receipt: terminal_receipt,
                }),
            }),
        };

        let encoded = state.to_bytes();
        assert!(encoded.len() <= ROOT_FUNDING_STATE_MAX_BYTES as usize);
    }

    #[test]
    fn genesis_is_exact_and_conflicting_reinitialization_rejects() {
        RootFundingStore::import(RootFundingData::default());
        let genesis = RootFundingRecord::default();
        assert_eq!(
            RootFundingStore::commit_genesis(genesis.clone()),
            Ok(RootFundingCommitOutcome::Committed)
        );
        assert_eq!(
            RootFundingStore::commit_genesis(genesis),
            Ok(RootFundingCommitOutcome::Existing)
        );
        assert_eq!(
            RootFundingStore::commit_genesis(RootFundingRecord {
                schema_version: ROOT_FUNDING_SCHEMA_VERSION,
                policy_generation: 1,
                historical_automatic_grants: 0,
                historical_automatic_cycles: Cycles::new(0),
                automatic_grants: 0,
                automatic_cycles: Cycles::new(0),
                current: None,
                last: Some(RootFundingTerminalOperationRecord {
                    request: crate::test_support::root_funding_request_fixture(1),
                    response: FleetRootFundingResponse::NoGrant(
                        canic_core::dto::fleet_funding::FleetRootFundingNoGrantReceipt {
                            request: crate::test_support::root_funding_request_fixture(1),
                            reason: canic_core::dto::fleet_funding::FleetRootFundingNoGrantReason::FundingDisabled,
                            decided_at_ns: 1,
                        },
                    ),
                    opened_at_ns: 1,
                    completed_at_ns: 1,
                }),
                rotation_current: None,
                rotation_last: None,
            }),
            Err(RootFundingCommitError::ConflictingState)
        );
    }

    #[test]
    fn cycles_remain_a_compact_fixed_width_stable_value() {
        let encoded =
            canic_core::cdk::serialize::serialize(&Cycles::new(u128::MAX)).expect("encode cycles");
        assert!(encoded.len() <= 32);
    }
}
