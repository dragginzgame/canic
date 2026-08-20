//! Test-only attached-cycles recovery probe.
//!
//! One Wasm serves either a minimal Coordinator or its exact root. It proves
//! intent-before-effect, caller-bound acceptance and zero-accept replay without
//! adding a production funding surface.

use candid::{CandidType, Deserialize, Principal, encode_one};
use ic_cdk::{
    api::{
        canister_self, cost_call, is_controller, msg_caller, msg_cycles_accept,
        msg_cycles_available,
    },
    call::Call,
    trap,
};
use std::cell::RefCell;

const ROOT_ACCEPT_METHOD: &str = "root_funding_probe_accept";

thread_local! {
    static ROLE: RefCell<Option<ProbeRole>> = const { RefCell::new(None) };
    static INTENT: RefCell<Option<GrantIntent>> = const { RefCell::new(None) };
    static RECEIPT: RefCell<Option<GrantReceipt>> = const { RefCell::new(None) };
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
enum ProbeInit {
    Coordinator,
    Root { coordinator: Principal },
}

#[derive(Clone)]
enum ProbeRole {
    Coordinator,
    Root { coordinator: Principal },
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
    };
    ROLE.with_borrow_mut(|stored| *stored = Some(role));
}

#[ic_cdk::update(name = "root_funding_probe_prepare")]
fn prepare(root: Principal, operation_id: [u8; 32], amount: u128) -> GrantIntent {
    require_controller();
    require_coordinator_role();
    if amount == 0 {
        trap("grant amount must be positive");
    }

    let request = GrantRequest {
        operation_id,
        amount,
    };
    let payload = encode_one(&request).unwrap_or_else(|_| trap("encode grant request"));
    let call_cost = cost_call(ROOT_ACCEPT_METHOD.len() as u64, payload.len() as u64);
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
        Some(current) if *current == candidate => current.clone(),
        Some(_) => trap("grant intent binding conflict"),
        None => {
            *stored = Some(candidate.clone());
            candidate
        }
    })
}

#[ic_cdk::update(name = "root_funding_probe_issue")]
async fn issue() -> IssueResult {
    require_controller();
    require_coordinator_role();
    issue_prepared().await
}

#[ic_cdk::update(name = "root_funding_probe_issue_then_trap")]
async fn issue_then_trap() -> IssueResult {
    require_controller();
    require_coordinator_role();
    let result = issue_prepared().await;
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

#[ic_cdk::update(name = "root_funding_probe_accept")]
fn accept(request: GrantRequest) -> RootAcceptResult {
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

        let accepted_cycles = msg_cycles_accept(request.amount);
        if accepted_cycles != request.amount {
            trap("fresh grant acceptance was not exact");
        }
        let receipt = GrantReceipt {
            operation_id: request.operation_id,
            coordinator,
            root: canister_self(),
            amount: request.amount,
        };
        *stored = Some(receipt.clone());
        RootAcceptResult::Accepted {
            receipt,
            accepted_cycles,
            replay: false,
        }
    })
}

async fn issue_prepared() -> IssueResult {
    let Some(intent) = INTENT.with_borrow(Clone::clone) else {
        return IssueResult::MissingIntent;
    };
    if let IntentState::Committed { receipt } = intent.state {
        return IssueResult::AlreadyCommitted { receipt };
    }

    let Ok(response) = Call::bounded_wait(intent.root, ROOT_ACCEPT_METHOD)
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
