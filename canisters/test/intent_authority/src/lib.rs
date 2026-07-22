//!
//! Minimal authority canister for intent reservation tests.
//!

use candid::{CandidType, Deserialize, Principal};
use canic::api::intent::{
    BeginReceiptBackedIntentInput, BeginReceiptBackedIntentResult, IntentResourceKey, OperationId,
    PayloadBinding, ReceiptBackedIntent, ReceiptBackedIntentApi, ReceiptBackedIntentState,
    SettleReceiptBackedIntentInput, SettleReceiptBackedIntentResult, TerminalEvidence,
    TerminalEvidenceDecision,
};
use ic_cdk::{query, update};

const RECEIPT_CAPACITY: u64 = 1;

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
enum ReceiptStateView {
    Pending,
    Committed {
        source_canister: Principal,
        fingerprint: [u8; 32],
    },
    RolledBack {
        source_canister: Principal,
        fingerprint: [u8; 32],
    },
}

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
struct ReceiptIntentView {
    payload_digest: [u8; 32],
    quantity: u64,
    revision: u64,
    state: ReceiptStateView,
}

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
enum ReceiptBeginStatus {
    Created,
    ExistingPending,
    ExistingCommitted,
    ExistingRolledBack,
    BindingConflict,
    ReplayWindowClosed {
        replay_deadline_ns: u64,
    },
    ReplayWindowTooLong {
        remaining_ns: u64,
        maximum_ns: u64,
    },
    CapacityExceeded {
        current_quantity: u64,
        requested_quantity: u64,
        limit: u64,
    },
    StoreCapacityReached {
        current_records: u64,
        limit: u64,
    },
}

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
struct ReceiptBeginView {
    status: ReceiptBeginStatus,
    intent: Option<ReceiptIntentView>,
}

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
enum ReceiptSettlementStatus {
    Settled,
    AlreadySettled,
    NotFound,
    RevisionConflict { actual_revision: u64 },
    BindingConflict,
}

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
struct ReceiptSettlementView {
    status: ReceiptSettlementStatus,
    intent: Option<ReceiptIntentView>,
}

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
enum ReceiptDecisionView {
    Committed,
    RolledBack,
}

#[ic_cdk::init]
fn init() {
    init_memory();
    ic_cdk::println!("intent_authority: init");
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    init_memory();
    ic_cdk::println!("intent_authority: post_upgrade memory initialized");
}

#[update]
fn begin_receipt(
    operation_seed: u8,
    payload_seed: u8,
    resource_seed: u8,
    quantity: u64,
    replay_deadline_ns: u64,
) -> Result<ReceiptBeginView, String> {
    init_memory();
    let operation_id = operation_id(operation_seed);
    let result = ReceiptBackedIntentApi::begin_or_load(&BeginReceiptBackedIntentInput {
        operation_id,
        payload_binding: payload_binding(payload_seed),
        resource_key: receipt_key(resource_seed)?,
        quantity,
        reservation_limit: RECEIPT_CAPACITY,
        replay_deadline_ns,
    })
    .map_err(|err| err.to_string())?;

    let status = match result {
        BeginReceiptBackedIntentResult::Created { .. } => ReceiptBeginStatus::Created,
        BeginReceiptBackedIntentResult::ExistingPending { .. } => {
            ReceiptBeginStatus::ExistingPending
        }
        BeginReceiptBackedIntentResult::ExistingCommitted { .. } => {
            ReceiptBeginStatus::ExistingCommitted
        }
        BeginReceiptBackedIntentResult::ExistingRolledBack { .. } => {
            ReceiptBeginStatus::ExistingRolledBack
        }
        BeginReceiptBackedIntentResult::BindingConflict => ReceiptBeginStatus::BindingConflict,
        BeginReceiptBackedIntentResult::ReplayWindowClosed { replay_deadline_ns } => {
            ReceiptBeginStatus::ReplayWindowClosed { replay_deadline_ns }
        }
        BeginReceiptBackedIntentResult::ReplayWindowTooLong {
            remaining_ns,
            maximum_ns,
        } => ReceiptBeginStatus::ReplayWindowTooLong {
            remaining_ns,
            maximum_ns,
        },
        BeginReceiptBackedIntentResult::CapacityExceeded {
            current_quantity,
            requested_quantity,
            limit,
        } => ReceiptBeginStatus::CapacityExceeded {
            current_quantity,
            requested_quantity,
            limit,
        },
        BeginReceiptBackedIntentResult::StoreCapacityReached {
            current_records,
            limit,
        } => ReceiptBeginStatus::StoreCapacityReached {
            current_records,
            limit,
        },
    };
    let intent = match &status {
        ReceiptBeginStatus::Created
        | ReceiptBeginStatus::ExistingPending
        | ReceiptBeginStatus::ExistingCommitted
        | ReceiptBeginStatus::ExistingRolledBack => load_receipt_view(operation_id)?,
        ReceiptBeginStatus::BindingConflict
        | ReceiptBeginStatus::ReplayWindowClosed { .. }
        | ReceiptBeginStatus::ReplayWindowTooLong { .. }
        | ReceiptBeginStatus::CapacityExceeded { .. }
        | ReceiptBeginStatus::StoreCapacityReached { .. } => None,
    };
    Ok(ReceiptBeginView { status, intent })
}

#[query]
fn load_receipt(operation_seed: u8) -> Result<Option<ReceiptIntentView>, String> {
    load_receipt_view(operation_id(operation_seed))
}

#[update]
fn settle_receipt(
    operation_seed: u8,
    payload_seed: u8,
    expected_revision: u64,
    decision: ReceiptDecisionView,
    source_canister: Principal,
    evidence_seed: u8,
) -> Result<ReceiptSettlementView, String> {
    init_memory();
    let operation_id = operation_id(operation_seed);
    let decision = match decision {
        ReceiptDecisionView::Committed => TerminalEvidenceDecision::Committed,
        ReceiptDecisionView::RolledBack => TerminalEvidenceDecision::RolledBack,
    };
    let result = ReceiptBackedIntentApi::settle_if_pending(&SettleReceiptBackedIntentInput {
        operation_id,
        expected_revision,
        expected_payload_binding: payload_binding(payload_seed),
        evidence: TerminalEvidence::new(source_canister, decision, [evidence_seed; 32]),
    })
    .map_err(|err| err.to_string())?;

    let status = match result {
        SettleReceiptBackedIntentResult::Settled { .. } => ReceiptSettlementStatus::Settled,
        SettleReceiptBackedIntentResult::AlreadySettled { .. } => {
            ReceiptSettlementStatus::AlreadySettled
        }
        SettleReceiptBackedIntentResult::NotFound => ReceiptSettlementStatus::NotFound,
        SettleReceiptBackedIntentResult::RevisionConflict { actual_revision } => {
            ReceiptSettlementStatus::RevisionConflict { actual_revision }
        }
        SettleReceiptBackedIntentResult::BindingConflict => {
            ReceiptSettlementStatus::BindingConflict
        }
    };
    let intent = match &status {
        ReceiptSettlementStatus::Settled | ReceiptSettlementStatus::AlreadySettled => {
            load_receipt_view(operation_id)?
        }
        ReceiptSettlementStatus::NotFound
        | ReceiptSettlementStatus::RevisionConflict { .. }
        | ReceiptSettlementStatus::BindingConflict => None,
    };
    Ok(ReceiptSettlementView { status, intent })
}

fn init_memory() {
    canic::api::runtime::MemoryRuntimeApi::bootstrap_registry()
        .expect("memory registry init should succeed");
}

fn receipt_key(seed: u8) -> Result<IntentResourceKey, String> {
    IntentResourceKey::try_new(format!("receipt_capacity:{seed}")).map_err(|err| err.to_string())
}

const fn operation_id(seed: u8) -> OperationId {
    OperationId::from_bytes([seed; 32])
}

const fn payload_binding(seed: u8) -> PayloadBinding {
    PayloadBinding::new([seed; 32])
}

fn load_receipt_view(operation_id: OperationId) -> Result<Option<ReceiptIntentView>, String> {
    ReceiptBackedIntentApi::load(operation_id)
        .map(|intent| intent.map(ReceiptIntentView::from))
        .map_err(|err| err.to_string())
}

impl From<ReceiptBackedIntent> for ReceiptIntentView {
    fn from(intent: ReceiptBackedIntent) -> Self {
        let state = match intent.state {
            ReceiptBackedIntentState::Pending => ReceiptStateView::Pending,
            ReceiptBackedIntentState::Committed { evidence } => ReceiptStateView::Committed {
                source_canister: evidence.source_canister,
                fingerprint: evidence.fingerprint,
            },
            ReceiptBackedIntentState::RolledBack { evidence } => ReceiptStateView::RolledBack {
                source_canister: evidence.source_canister,
                fingerprint: evidence.fingerprint,
            },
        };
        Self {
            payload_digest: intent.payload_binding.digest,
            quantity: intent.quantity,
            revision: intent.revision,
            state,
        }
    }
}
