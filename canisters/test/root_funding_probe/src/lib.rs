//! Test-only attached-cycles recovery probe.
//!
//! One Wasm serves minimal Coordinator, Root, Ledger and CMC proof roles. It
//! proves attached-cycle atomicity, exact replay and bounded request/refill
//! recovery costs without adding a production funding surface.

use candid::{CandidType, Deserialize, Principal, encode_one};
use ic_cdk::{
    api::{
        canister_cycle_balance, canister_self, cost_call, is_controller, msg_caller,
        msg_cycles_accept, msg_cycles_available,
    },
    call::Call,
    trap,
};
use std::cell::RefCell;

const ROOT_ACCEPT_METHOD: &str = "root_funding_probe_accept";
const ROOT_ACCEPT_THEN_TRAP_METHOD: &str = "root_funding_probe_accept_then_trap";
const COORDINATOR_HANDLE_REQUEST_METHOD: &str = "root_funding_probe_handle_request";
const COORDINATOR_FUNDING_COMMAND_METHOD: &str = "canic_coordinator_command";
const ROOT_FUNDING_COMMAND_METHOD: &str = "canic_root_command";
const FINAL_FUNDING_COMMAND_MAX_ENCODED_BYTES: u64 = 16_384;
const LEDGER_FEE_METHOD: &str = "icrc1_fee";
const LEDGER_DECIMALS_METHOD: &str = "icrc1_decimals";
const LEDGER_TRANSFER_METHOD: &str = "icrc1_transfer";
const CMC_RATE_METHOD: &str = "get_icp_xdr_conversion_rate";
const CMC_NOTIFY_METHOD: &str = "notify_top_up";

thread_local! {
    static ROLE: RefCell<Option<ProbeRole>> = const { RefCell::new(None) };
    static INTENT: RefCell<Option<GrantIntent>> = const { RefCell::new(None) };
    static RECEIPT: RefCell<Option<GrantReceipt>> = const { RefCell::new(None) };
    static ACCEPTANCE_OBSERVATION: RefCell<Option<AcceptanceObservation>> = const { RefCell::new(None) };
    static EMERGENCY_OPERATION: RefCell<Option<EmergencyRefillOperation>> = const { RefCell::new(None) };
    static LEDGER_TRANSFER: RefCell<Option<LedgerTransferReceipt>> = const { RefCell::new(None) };
    static CMC_NOTIFY: RefCell<Option<CmcNotifyReceipt>> = const { RefCell::new(None) };
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
enum ProbeInit {
    Coordinator,
    Root { coordinator: Principal },
    EmergencyRoot { ledger: Principal, cmc: Principal },
    Ledger,
    Cmc,
}

#[derive(Clone)]
enum ProbeRole {
    Coordinator,
    Root { coordinator: Principal },
    EmergencyRoot { ledger: Principal, cmc: Principal },
    Ledger,
    Cmc,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct GrantRequest {
    operation_id: [u8; 32],
    amount: u128,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct GrantReceipt {
    operation_id: [u8; 32],
    coordinator: Principal,
    root: Principal,
    amount: u128,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct AcceptanceObservation {
    balance_before_accept: u128,
    balance_after_accept: u128,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct RootRequestObservation {
    result: IssueResult,
    call_cost: u128,
    balance_before_request: u128,
    balance_after_request: u128,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct EmergencyTransferRequest {
    operation_id: [u8; 32],
    from_subaccount: Option<[u8; 32]>,
    to_owner: Principal,
    to_subaccount: [u8; 32],
    amount_e8s: u64,
    fee_e8s: u64,
    memo: Vec<u8>,
    created_at_time_ns: u64,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct CmcNotifyRequest {
    block_index: u64,
    canister_id: Principal,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct LedgerTransferReceipt {
    operation_id: [u8; 32],
    block_index: u64,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
enum LedgerTransferResult {
    Transferred { block_index: u64 },
    Duplicate { block_index: u64 },
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct CmcNotifyReceipt {
    request: CmcNotifyRequest,
    cycles_sent: u128,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
enum EmergencyRefillPhase {
    Prepared,
    Transferred { block_index: u64 },
    Completed { block_index: u64, cycles_sent: u128 },
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct EmergencyRefillOperation {
    transfer: EmergencyTransferRequest,
    max_call_cost: u128,
    phase: EmergencyRefillPhase,
}

#[derive(Clone, Copy)]
enum EmergencyTrapPoint {
    None,
    TransferCallback,
    NotifyCallback,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
enum IntentState {
    Prepared,
    Committed { receipt: GrantReceipt },
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct GrantIntent {
    root: Principal,
    request: GrantRequest,
    call_cost: u128,
    call_reservation: u128,
    state: IntentState,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
enum RootAcceptResult {
    Accepted {
        receipt: GrantReceipt,
        accepted_cycles: u128,
        replay: bool,
    },
    CallerDenied,
    AttachedAmountMismatch,
    BindingConflict,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
enum IssueResult {
    Committed {
        receipt: GrantReceipt,
        accepted_cycles: u128,
        replay: bool,
    },
    AlreadyCommitted {
        receipt: GrantReceipt,
    },
    MissingIntent,
    CallFailed,
    RootDenied,
    RootBindingConflict,
}

#[ic_cdk::init]
fn init(arg: ProbeInit) {
    let role = match arg {
        ProbeInit::Coordinator => ProbeRole::Coordinator,
        ProbeInit::Root { coordinator } => ProbeRole::Root { coordinator },
        ProbeInit::EmergencyRoot { ledger, cmc } => ProbeRole::EmergencyRoot { ledger, cmc },
        ProbeInit::Ledger => ProbeRole::Ledger,
        ProbeInit::Cmc => ProbeRole::Cmc,
    };
    ROLE.with_borrow_mut(|stored| *stored = Some(role));
}

#[ic_cdk::update(name = "root_funding_probe_prepare")]
fn prepare(root: Principal, operation_id: [u8; 32], amount: u128) -> GrantIntent {
    require_controller();
    require_coordinator_role();
    prepare_intent(root, operation_id, amount)
}

fn prepare_intent(root: Principal, operation_id: [u8; 32], amount: u128) -> GrantIntent {
    if amount == 0 {
        trap("grant amount must be positive");
    }

    let request = GrantRequest {
        operation_id,
        amount,
    };
    let payload = encode_one(&request).unwrap_or_else(|_| trap("encode grant request"));
    if payload.len() as u64 > FINAL_FUNDING_COMMAND_MAX_ENCODED_BYTES {
        trap("grant request exceeds final command payload bound");
    }
    let call_cost = cost_call(
        ROOT_FUNDING_COMMAND_METHOD.len() as u64,
        FINAL_FUNDING_COMMAND_MAX_ENCODED_BYTES,
    );
    let call_reservation = amount
        .checked_add(call_cost)
        .unwrap_or_else(|| trap("grant call reservation overflow"));
    let candidate = GrantIntent {
        root,
        request,
        call_cost,
        call_reservation,
        state: IntentState::Prepared,
    };

    INTENT.with_borrow_mut(|stored| match stored {
        Some(current)
            if current.root == candidate.root
                && current.request == candidate.request
                && current.call_cost == candidate.call_cost
                && current.call_reservation == candidate.call_reservation =>
        {
            current.clone()
        }
        Some(_) => trap("grant intent binding conflict"),
        None => {
            *stored = Some(candidate.clone());
            candidate
        }
    })
}

#[ic_cdk::update(name = "root_funding_probe_handle_request")]
async fn handle_request(request: GrantRequest) -> IssueResult {
    require_coordinator_role();
    let root = msg_caller();
    let _ = prepare_intent(root, request.operation_id, request.amount);
    issue_prepared(ROOT_ACCEPT_METHOD).await
}

#[ic_cdk::update(name = "root_funding_probe_request")]
async fn request() -> RootRequestObservation {
    let coordinator = require_root_role();
    let request = GrantRequest {
        operation_id: [0x17; 32],
        amount: 1_000_000_000_000,
    };
    let payload = encode_one(&request).unwrap_or_else(|_| trap("encode root grant request"));
    if payload.len() as u64 > FINAL_FUNDING_COMMAND_MAX_ENCODED_BYTES {
        trap("root grant request exceeds final command payload bound");
    }
    let call_cost = cost_call(
        COORDINATOR_FUNDING_COMMAND_METHOD.len() as u64,
        FINAL_FUNDING_COMMAND_MAX_ENCODED_BYTES,
    );
    let balance_before_request = canister_cycle_balance();
    let result = match Call::bounded_wait(coordinator, COORDINATOR_HANDLE_REQUEST_METHOD)
        .with_arg(request)
        .await
    {
        Ok(response) => response
            .candid::<IssueResult>()
            .unwrap_or(IssueResult::RootBindingConflict),
        Err(_) => IssueResult::CallFailed,
    };
    RootRequestObservation {
        result,
        call_cost,
        balance_before_request,
        balance_after_request: canister_cycle_balance(),
    }
}

#[ic_cdk::update(name = "root_funding_probe_emergency_refill")]
async fn emergency_refill() -> EmergencyRefillOperation {
    require_controller();
    advance_emergency_refill(EmergencyTrapPoint::None).await
}

#[ic_cdk::update(name = "root_funding_probe_emergency_refill_transfer_then_trap")]
async fn emergency_refill_transfer_then_trap() -> EmergencyRefillOperation {
    require_controller();
    advance_emergency_refill(EmergencyTrapPoint::TransferCallback).await
}

#[ic_cdk::update(name = "root_funding_probe_emergency_refill_notify_then_trap")]
async fn emergency_refill_notify_then_trap() -> EmergencyRefillOperation {
    require_controller();
    advance_emergency_refill(EmergencyTrapPoint::NotifyCallback).await
}

#[ic_cdk::query(name = "root_funding_probe_emergency_operation")]
fn emergency_operation() -> Option<EmergencyRefillOperation> {
    require_emergency_root_role();
    EMERGENCY_OPERATION.with_borrow(Clone::clone)
}

#[ic_cdk::update(name = "icrc1_fee")]
fn ledger_fee() -> u64 {
    require_ledger_role();
    10_000
}

#[ic_cdk::update(name = "icrc1_decimals")]
fn ledger_decimals() -> u8 {
    require_ledger_role();
    8
}

#[ic_cdk::update(name = "icrc1_transfer")]
fn ledger_transfer(request: EmergencyTransferRequest) -> LedgerTransferResult {
    require_ledger_role();
    LEDGER_TRANSFER.with_borrow_mut(|stored| {
        if let Some(receipt) = stored.as_ref() {
            if receipt.operation_id != request.operation_id {
                trap("ledger transfer operation binding conflict");
            }
            return LedgerTransferResult::Duplicate {
                block_index: receipt.block_index,
            };
        }
        let receipt = LedgerTransferReceipt {
            operation_id: request.operation_id,
            block_index: 77,
        };
        *stored = Some(receipt.clone());
        LedgerTransferResult::Transferred {
            block_index: receipt.block_index,
        }
    })
}

#[ic_cdk::update(name = "get_icp_xdr_conversion_rate")]
fn cmc_rate() -> u64 {
    require_cmc_role();
    1_000_000
}

#[ic_cdk::update(name = "notify_top_up")]
fn cmc_notify(request: CmcNotifyRequest) -> u128 {
    require_cmc_role();
    CMC_NOTIFY.with_borrow_mut(|stored| {
        if let Some(receipt) = stored.as_ref() {
            if receipt.request != request {
                trap("CMC notification binding conflict");
            }
            return receipt.cycles_sent;
        }
        let receipt = CmcNotifyReceipt {
            request,
            cycles_sent: 2_000_000_000_000,
        };
        *stored = Some(receipt.clone());
        receipt.cycles_sent
    })
}

async fn advance_emergency_refill(trap_point: EmergencyTrapPoint) -> EmergencyRefillOperation {
    let mut operation = prepare_emergency_refill().await;
    if matches!(operation.phase, EmergencyRefillPhase::Prepared) {
        let (ledger, _) = require_emergency_root_role();
        let response = Call::bounded_wait(ledger, LEDGER_TRANSFER_METHOD)
            .with_arg(operation.transfer.clone())
            .await
            .unwrap_or_else(|_| trap("ledger transfer call failed"));
        let transfer = response
            .candid::<LedgerTransferResult>()
            .unwrap_or_else(|_| trap("decode ledger transfer response"));
        if matches!(trap_point, EmergencyTrapPoint::TransferCallback) {
            trap("intentional trap after ledger transfer response");
        }
        let block_index = match transfer {
            LedgerTransferResult::Transferred { block_index }
            | LedgerTransferResult::Duplicate { block_index } => block_index,
        };
        operation.phase = EmergencyRefillPhase::Transferred { block_index };
        store_emergency_operation(&operation);
    }

    let EmergencyRefillPhase::Transferred { block_index } = operation.phase else {
        return operation;
    };
    let (_, cmc) = require_emergency_root_role();
    if operation.transfer.to_owner != cmc {
        trap("emergency CMC transfer authority changed");
    }
    let notify = CmcNotifyRequest {
        block_index,
        canister_id: canister_self(),
    };
    let response = Call::bounded_wait(cmc, CMC_NOTIFY_METHOD)
        .with_arg(notify)
        .await
        .unwrap_or_else(|_| trap("CMC notify call failed"));
    let cycles_sent = response
        .candid::<u128>()
        .unwrap_or_else(|_| trap("decode CMC notify response"));
    if matches!(trap_point, EmergencyTrapPoint::NotifyCallback) {
        trap("intentional trap after CMC notify response");
    }
    operation.phase = EmergencyRefillPhase::Completed {
        block_index,
        cycles_sent,
    };
    store_emergency_operation(&operation);
    operation
}

async fn prepare_emergency_refill() -> EmergencyRefillOperation {
    if let Some(operation) = EMERGENCY_OPERATION.with_borrow(Clone::clone) {
        return operation;
    }
    let (ledger, cmc) = require_emergency_root_role();
    let fee_response = Call::bounded_wait(ledger, LEDGER_FEE_METHOD)
        .with_arg(())
        .await
        .unwrap_or_else(|_| trap("ledger fee call failed"));
    let fee_e8s = fee_response
        .candid::<u64>()
        .unwrap_or_else(|_| trap("decode ledger fee"));
    let decimals_response = Call::bounded_wait(ledger, LEDGER_DECIMALS_METHOD)
        .with_arg(())
        .await
        .unwrap_or_else(|_| trap("ledger decimals call failed"));
    if decimals_response
        .candid::<u8>()
        .unwrap_or_else(|_| trap("decode ledger decimals"))
        != 8
    {
        trap("unexpected ledger decimals");
    }
    let rate_response = Call::bounded_wait(cmc, CMC_RATE_METHOD)
        .with_arg(())
        .await
        .unwrap_or_else(|_| trap("CMC rate call failed"));
    let _rate = rate_response
        .candid::<u64>()
        .unwrap_or_else(|_| trap("decode CMC rate"));

    let transfer = EmergencyTransferRequest {
        operation_id: [0x81; 32],
        from_subaccount: None,
        to_owner: cmc,
        to_subaccount: [0x42; 32],
        amount_e8s: 100_000_000,
        fee_e8s,
        memo: b"TPUP\0\0\0\0".to_vec(),
        created_at_time_ns: ic_cdk::api::time(),
    };
    let notify = CmcNotifyRequest {
        block_index: 77,
        canister_id: canister_self(),
    };
    let empty_payload = encode_one(()).unwrap_or_else(|_| trap("encode empty call payload"));
    let transfer_payload =
        encode_one(&transfer).unwrap_or_else(|_| trap("encode transfer payload"));
    let notify_payload = encode_one(&notify).unwrap_or_else(|_| trap("encode notify payload"));
    let max_call_cost = [
        cost_call(LEDGER_FEE_METHOD.len() as u64, empty_payload.len() as u64),
        cost_call(
            LEDGER_DECIMALS_METHOD.len() as u64,
            empty_payload.len() as u64,
        ),
        cost_call(CMC_RATE_METHOD.len() as u64, empty_payload.len() as u64),
        cost_call(
            LEDGER_TRANSFER_METHOD.len() as u64,
            transfer_payload.len() as u64,
        ),
        cost_call(CMC_NOTIFY_METHOD.len() as u64, notify_payload.len() as u64),
    ]
    .into_iter()
    .max()
    .unwrap_or_else(|| trap("emergency call-cost set is empty"));
    let operation = EmergencyRefillOperation {
        transfer,
        max_call_cost,
        phase: EmergencyRefillPhase::Prepared,
    };
    store_emergency_operation(&operation);
    operation
}

fn store_emergency_operation(operation: &EmergencyRefillOperation) {
    EMERGENCY_OPERATION.with_borrow_mut(|stored| *stored = Some(operation.clone()));
}

#[ic_cdk::update(name = "root_funding_probe_issue")]
async fn issue() -> IssueResult {
    require_controller();
    require_coordinator_role();
    issue_prepared(ROOT_ACCEPT_METHOD).await
}

#[ic_cdk::update(name = "root_funding_probe_issue_root_accept_then_trap")]
async fn issue_root_accept_then_trap() -> IssueResult {
    require_controller();
    require_coordinator_role();
    issue_prepared(ROOT_ACCEPT_THEN_TRAP_METHOD).await
}

#[ic_cdk::update(name = "root_funding_probe_issue_then_trap")]
async fn issue_then_trap() -> IssueResult {
    require_controller();
    require_coordinator_role();
    let result = issue_prepared(ROOT_ACCEPT_METHOD).await;
    if matches!(result, IssueResult::Committed { .. }) {
        trap("intentional response-loss boundary after root receipt");
    }
    result
}

#[ic_cdk::query(name = "root_funding_probe_intent")]
fn intent() -> Option<GrantIntent> {
    require_coordinator_role();
    INTENT.with_borrow(Clone::clone)
}

#[ic_cdk::query(name = "root_funding_probe_receipt")]
fn receipt() -> Option<GrantReceipt> {
    require_root_role();
    RECEIPT.with_borrow(Clone::clone)
}

#[ic_cdk::query(name = "root_funding_probe_acceptance_observation")]
fn acceptance_observation() -> Option<AcceptanceObservation> {
    require_root_role();
    ACCEPTANCE_OBSERVATION.with_borrow(Clone::clone)
}

#[ic_cdk::update(name = "root_funding_probe_accept")]
fn accept(request: GrantRequest) -> RootAcceptResult {
    accept_request(request)
}

#[ic_cdk::update(name = "root_funding_probe_accept_then_trap")]
fn accept_then_trap(request: GrantRequest) -> RootAcceptResult {
    let _ = accept_request(request);
    trap("intentional trap after exact grant acceptance and receipt persistence");
}

fn accept_request(request: GrantRequest) -> RootAcceptResult {
    let coordinator = require_root_role();
    if msg_caller() != coordinator {
        return RootAcceptResult::CallerDenied;
    }
    if msg_cycles_available() != request.amount {
        return RootAcceptResult::AttachedAmountMismatch;
    }

    RECEIPT.with_borrow_mut(|stored| {
        if let Some(receipt) = stored.as_ref() {
            if receipt.operation_id != request.operation_id
                || receipt.amount != request.amount
                || receipt.coordinator != coordinator
                || receipt.root != canister_self()
            {
                return RootAcceptResult::BindingConflict;
            }
            return RootAcceptResult::Accepted {
                receipt: receipt.clone(),
                accepted_cycles: 0,
                replay: true,
            };
        }

        let balance_before_accept = canister_cycle_balance();
        let accepted_cycles = msg_cycles_accept(request.amount);
        if accepted_cycles != request.amount {
            trap("fresh grant acceptance was not exact");
        }
        let balance_after_accept = canister_cycle_balance();
        let receipt = GrantReceipt {
            operation_id: request.operation_id,
            coordinator,
            root: canister_self(),
            amount: request.amount,
        };
        *stored = Some(receipt.clone());
        ACCEPTANCE_OBSERVATION.with_borrow_mut(|observation| {
            *observation = Some(AcceptanceObservation {
                balance_before_accept,
                balance_after_accept,
            });
        });
        RootAcceptResult::Accepted {
            receipt,
            accepted_cycles,
            replay: false,
        }
    })
}

async fn issue_prepared(root_method: &str) -> IssueResult {
    let Some(intent) = INTENT.with_borrow(Clone::clone) else {
        return IssueResult::MissingIntent;
    };
    if let IntentState::Committed { receipt } = intent.state {
        return IssueResult::AlreadyCommitted { receipt };
    }

    let Ok(response) = Call::bounded_wait(intent.root, root_method)
        .with_arg(intent.request.clone())
        .with_cycles(intent.request.amount)
        .await
    else {
        return IssueResult::CallFailed;
    };
    let Ok(root_result) = response.candid::<RootAcceptResult>() else {
        return IssueResult::RootBindingConflict;
    };
    let RootAcceptResult::Accepted {
        receipt,
        accepted_cycles,
        replay,
    } = root_result
    else {
        return match root_result {
            RootAcceptResult::CallerDenied => IssueResult::RootDenied,
            RootAcceptResult::AttachedAmountMismatch | RootAcceptResult::BindingConflict => {
                IssueResult::RootBindingConflict
            }
            RootAcceptResult::Accepted { .. } => unreachable!(),
        };
    };
    if receipt.operation_id != intent.request.operation_id
        || receipt.amount != intent.request.amount
        || receipt.coordinator != canister_self()
        || receipt.root != intent.root
        || (replay && accepted_cycles != 0)
        || (!replay && accepted_cycles != intent.request.amount)
    {
        return IssueResult::RootBindingConflict;
    }

    INTENT.with_borrow_mut(|stored| {
        let Some(current) = stored.as_mut() else {
            trap("grant intent disappeared during call");
        };
        if current.root != intent.root || current.request != intent.request {
            trap("grant intent changed during call");
        }
        current.state = IntentState::Committed {
            receipt: receipt.clone(),
        };
    });
    IssueResult::Committed {
        receipt,
        accepted_cycles,
        replay,
    }
}

fn require_controller() {
    if !is_controller(&msg_caller()) {
        trap("controller required");
    }
}

fn require_coordinator_role() {
    ROLE.with_borrow(|role| {
        if !matches!(role, Some(ProbeRole::Coordinator)) {
            trap("Coordinator probe role required");
        }
    });
}

fn require_root_role() -> Principal {
    ROLE.with_borrow(|role| match role {
        Some(ProbeRole::Root { coordinator }) => *coordinator,
        _ => trap("root probe role required"),
    })
}

fn require_emergency_root_role() -> (Principal, Principal) {
    ROLE.with_borrow(|role| match role {
        Some(ProbeRole::EmergencyRoot { ledger, cmc }) => (*ledger, *cmc),
        _ => trap("emergency root probe role required"),
    })
}

fn require_ledger_role() {
    ROLE.with_borrow(|role| {
        if !matches!(role, Some(ProbeRole::Ledger)) {
            trap("ledger probe role required");
        }
    });
}

fn require_cmc_role() {
    ROLE.with_borrow(|role| {
        if !matches!(role, Some(ProbeRole::Cmc)) {
            trap("CMC probe role required");
        }
    });
}
