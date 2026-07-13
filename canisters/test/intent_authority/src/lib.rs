//!
//! Minimal authority canister for intent reservation tests.
//!

use candid::{CandidType, Deserialize, Principal};
use canic::api::{
    ic::Call,
    intent::{
        BeginLocalIntentInput, BeginReceiptBackedIntentInput, BeginReceiptBackedIntentResult,
        IntentResourceKey, LocalIntentApi, OperationId, PayloadBinding, ReceiptBackedIntent,
        ReceiptBackedIntentApi, ReceiptBackedIntentState, SettleReceiptBackedIntentInput,
        SettleReceiptBackedIntentResult, TerminalEvidence, TerminalEvidenceDecision,
    },
};
use ic_cdk::{query, update};
use std::cell::RefCell;

const CAPACITY: u64 = 1;
const RECEIPT_CAPACITY: u64 = 1;

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
enum ReceiptStateView {
    Pending,
    Committed { fingerprint: [u8; 32] },
    RolledBack { fingerprint: [u8; 32] },
}

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
struct ReceiptIntentView {
    payload_digest: [u8; 32],
    quantity: u64,
    revision: u64,
    state: ReceiptStateView,
}

thread_local! {
    static EXTERNAL: RefCell<Option<Principal>> = const { RefCell::new(None) };
}

#[ic_cdk::init]
fn init(external: Principal) {
    init_memory();
    ic_cdk::println!("intent_authority: init external={external}");
    EXTERNAL.with(|cell| *cell.borrow_mut() = Some(external));
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    init_memory();
    ic_cdk::println!("intent_authority: post_upgrade memory initialized");
}

#[update]
async fn buy(qty: u64) -> Result<(), String> {
    // Idempotent bootstrap guard for custom test canister wiring.
    init_memory();
    ic_cdk::println!("intent_authority: buy start qty={qty}");

    let external = external_principal()?;
    ic_cdk::println!("intent_authority: call external perform {}", external);
    let call_builder = Call::unbounded_wait(external, "perform")
        .with_arg(())
        .map_err(|err| err.to_string())?;
    let intent_id = LocalIntentApi::begin(BeginLocalIntentInput {
        resource_key: intent_key()?,
        quantity: qty,
        ttl_secs: Some(60),
        reservation_limit: Some(CAPACITY),
    })
    .map_err(|err| err.to_string())?;
    let call_result = call_builder.execute().await;

    match call_result {
        Ok(_) => {
            LocalIntentApi::commit(intent_id).map_err(|err| err.to_string())?;
            ic_cdk::println!("intent_authority: external ok");
            Ok(())
        }
        Err(call_err) => {
            LocalIntentApi::rollback(intent_id).map_err(|rollback_err| {
                format!("external call failed: {call_err}; intent rollback failed: {rollback_err}")
            })?;
            ic_cdk::println!("intent_authority: external failed err={call_err}");
            Err(format!("external call failed: {call_err}"))
        }
    }
}

#[update]
fn begin_receipt(
    operation_seed: u8,
    payload_seed: u8,
    quantity: u64,
) -> Result<Option<ReceiptIntentView>, String> {
    init_memory();
    let operation_id = operation_id(operation_seed);
    let result = ReceiptBackedIntentApi::begin_or_load(&BeginReceiptBackedIntentInput {
        operation_id,
        payload_binding: payload_binding(payload_seed),
        resource_key: receipt_key()?,
        quantity,
        reservation_limit: RECEIPT_CAPACITY,
    })
    .map_err(|err| err.to_string())?;

    match result {
        BeginReceiptBackedIntentResult::Created { .. }
        | BeginReceiptBackedIntentResult::ExistingPending { .. }
        | BeginReceiptBackedIntentResult::ExistingCommitted { .. }
        | BeginReceiptBackedIntentResult::ExistingRolledBack { .. } => {
            load_receipt_view(operation_id)
        }
        BeginReceiptBackedIntentResult::BindingConflict
        | BeginReceiptBackedIntentResult::CapacityExceeded { .. }
        | BeginReceiptBackedIntentResult::StoreCapacityReached { .. } => Ok(None),
    }
}

#[query]
fn load_receipt(operation_seed: u8) -> Result<Option<ReceiptIntentView>, String> {
    load_receipt_view(operation_id(operation_seed))
}

#[update]
fn commit_receipt(
    operation_seed: u8,
    payload_seed: u8,
    evidence_seed: u8,
) -> Result<Option<ReceiptIntentView>, String> {
    init_memory();
    let operation_id = operation_id(operation_seed);
    let result = ReceiptBackedIntentApi::settle_if_pending(&SettleReceiptBackedIntentInput {
        operation_id,
        expected_revision: 1,
        expected_payload_binding: payload_binding(payload_seed),
        evidence: TerminalEvidence::new(
            canic::cdk::api::canister_self(),
            TerminalEvidenceDecision::Committed,
            [evidence_seed; 32],
        ),
    })
    .map_err(|err| err.to_string())?;

    match result {
        SettleReceiptBackedIntentResult::Settled { .. }
        | SettleReceiptBackedIntentResult::AlreadySettled { .. } => load_receipt_view(operation_id),
        SettleReceiptBackedIntentResult::NotFound
        | SettleReceiptBackedIntentResult::RevisionConflict { .. }
        | SettleReceiptBackedIntentResult::BindingConflict => Ok(None),
    }
}

fn init_memory() {
    canic::api::runtime::MemoryRuntimeApi::bootstrap_registry()
        .expect("memory registry init should succeed");
}

fn intent_key() -> Result<IntentResourceKey, String> {
    IntentResourceKey::try_new("capacity")
}

fn receipt_key() -> Result<IntentResourceKey, String> {
    IntentResourceKey::try_new("receipt_capacity")
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
                fingerprint: evidence.fingerprint,
            },
            ReceiptBackedIntentState::RolledBack { evidence } => ReceiptStateView::RolledBack {
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

fn external_principal() -> Result<Principal, String> {
    EXTERNAL
        .with(|cell| *cell.borrow())
        .ok_or_else(|| "external canister not initialized".to_string())
}
