//!
//! Minimal authority canister for intent reservation tests.
//!

mod codec;
mod protocol;

use canic::api::intent::{
    BeginReceiptBackedIntentInput, BeginReceiptBackedIntentResult, IntentResourceKey, OperationId,
    PayloadBinding, ReceiptBackedIntent, ReceiptBackedIntentApi, ReceiptBackedIntentState,
    SettleReceiptBackedIntentInput, SettleReceiptBackedIntentResult, TerminalEvidence,
    TerminalEvidenceDecision,
};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

const RECEIPT_CAPACITY: u64 = 1;

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
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

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
struct ReceiptIntentView {
    payload_digest: [u8; 32],
    quantity: u64,
    revision: u64,
    state: ReceiptStateView,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
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

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
struct ReceiptBeginView {
    status: ReceiptBeginStatus,
    intent: Option<ReceiptIntentView>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
enum ReceiptSettlementStatus {
    Settled,
    AlreadySettled,
    NotFound,
    RevisionConflict { actual_revision: u64 },
    BindingConflict,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
struct ReceiptSettlementView {
    status: ReceiptSettlementStatus,
    intent: Option<ReceiptIntentView>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
enum ReceiptDecisionView {
    Committed,
    RolledBack,
}

#[ic_platform::cdk::init]
fn init() {
    init_timers();
    init_memory();
    ic_platform::cdk::println!("intent_authority: init");
}

#[ic_platform::cdk::post_upgrade]
fn post_upgrade() {
    init_timers();
    init_memory();
    ic_platform::cdk::println!("intent_authority: post_upgrade memory initialized");
}

canic::__canic_register_raw_wire_schema!("begin_receipt", Update, protocol::BEGIN, protocol::BeginRequest, Result<ReceiptBeginView, String>, protocol::LIMITS, protocol::LIMITS);
#[ic_platform::cdk::update(
    decode_with = "codec::begin::decode",
    encode_with = "codec::begin::encode"
)]
fn begin_receipt(request: protocol::BeginRequest) -> Result<ReceiptBeginView, String> {
    let protocol::BeginRequest {
        operation_seed,
        payload_seed,
        resource_seed,
        quantity,
        replay_deadline_ns,
    } = request;
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

canic::__canic_register_raw_wire_schema!(
    "load_receipt",
    Query,
    protocol::LOAD,
    u8,
    Result<Option<ReceiptIntentView>, String>,
    protocol::LIMITS,
    protocol::LIMITS
);
#[ic_platform::cdk::query(
    decode_with = "codec::load::decode",
    encode_with = "codec::load::encode"
)]
fn load_receipt(operation_seed: u8) -> Result<Option<ReceiptIntentView>, String> {
    load_receipt_view(operation_id(operation_seed))
}

canic::__canic_register_raw_wire_schema!("settle_receipt", Update, protocol::SETTLE, protocol::SettleRequest, Result<ReceiptSettlementView, String>, protocol::LIMITS, protocol::LIMITS);
#[ic_platform::cdk::update(
    decode_with = "codec::settle::decode",
    encode_with = "codec::settle::encode"
)]
fn settle_receipt(request: protocol::SettleRequest) -> Result<ReceiptSettlementView, String> {
    let protocol::SettleRequest {
        operation_seed,
        payload_seed,
        expected_revision,
        decision,
        source_canister,
        evidence_seed,
    } = request;
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
        evidence: TerminalEvidence::new(source_canister.into(), decision, [evidence_seed; 32]),
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
    canic::memory::bootstrap(canic::memory::framework())
        .expect("memory registry init should succeed");
}

fn init_timers() {
    canic::__internal::core::api::timer::TimerApi::initialize_nonroot_runtime_required();
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
                source_canister: evidence.source_canister.into(),
                fingerprint: evidence.fingerprint,
            },
            ReceiptBackedIntentState::RolledBack { evidence } => ReceiptStateView::RolledBack {
                source_canister: evidence.source_canister.into(),
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
