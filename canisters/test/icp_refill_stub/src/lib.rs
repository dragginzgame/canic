//! Test-only ICRC-1 Ledger and CMC boundary for production Root refill journeys.
//!
//! One Wasm serves either role. The Ledger retains one exact transfer and
//! returns the canonical duplicate result on replay. The CMC retains one exact
//! notification and performs one real management-canister cycles deposit.

use candid::{CandidType, Deserialize, Nat, Principal};
use ic_cdk::{api::msg_caller, call::Call, trap};
use std::cell::RefCell;

const LEDGER_FEE_E8S: u64 = 10_000;
const ICP_DECIMALS: u8 = 8;
const BLOCK_INDEX: u64 = 77;

thread_local! {
    static STATE: RefCell<Option<StubState>> = const { RefCell::new(None) };
}

#[derive(CandidType, Clone, Debug, Deserialize)]
enum StubInit {
    Ledger {
        balance_e8s: u64,
    },
    Cmc {
        xdr_permyriad_per_icp: u64,
        cycles_per_notify: u128,
    },
}

#[derive(Clone, Debug)]
enum StubState {
    Ledger {
        balance_e8s: u64,
        transfer: Option<TransferReceipt>,
    },
    Cmc {
        xdr_permyriad_per_icp: u64,
        cycles_per_notify: u128,
        notification: Option<NotifyTopUpArg>,
    },
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
struct Icrc1Account {
    owner: Principal,
    subaccount: Option<[u8; 32]>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct TransferArg {
    from_subaccount: Option<[u8; 32]>,
    to: Icrc1Account,
    fee: Option<Nat>,
    created_at_time: Option<u64>,
    memo: Option<Memo>,
    amount: Nat,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
struct Memo(Vec<u8>);

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
enum TransferError {
    BadFee { expected_fee: Nat },
    InsufficientFunds { balance: Nat },
    Duplicate { duplicate_of: Nat },
    GenericError { error_code: Nat, message: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TransferReceipt {
    caller: Principal,
    request: TransferArg,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct NotifyTopUpArg {
    block_index: u64,
    canister_id: Principal,
}

#[derive(CandidType, Clone, Debug, Deserialize)]
enum NotifyTopUpError {
    Other {
        error_code: u64,
        error_message: String,
    },
}

#[derive(CandidType, Clone, Debug, Deserialize)]
struct IcpXdrConversionRate {
    xdr_permyriad_per_icp: u64,
    timestamp_seconds: u64,
}

#[derive(CandidType, Clone, Debug, Deserialize)]
struct IcpXdrConversionRateResponse {
    data: IcpXdrConversionRate,
    hash_tree: Vec<u8>,
    certificate: Vec<u8>,
}

#[derive(CandidType)]
struct CanisterIdRecord {
    canister_id: Principal,
}

#[ic_cdk::init]
fn init(args: StubInit) {
    let state = match args {
        StubInit::Ledger { balance_e8s } => StubState::Ledger {
            balance_e8s,
            transfer: None,
        },
        StubInit::Cmc {
            xdr_permyriad_per_icp,
            cycles_per_notify,
        } => StubState::Cmc {
            xdr_permyriad_per_icp,
            cycles_per_notify,
            notification: None,
        },
    };
    STATE.with_borrow_mut(|stored| *stored = Some(state));
}

#[ic_cdk::update(name = "icrc1_fee")]
fn icrc1_fee() -> Nat {
    require_ledger();
    Nat::from(LEDGER_FEE_E8S)
}

#[ic_cdk::update(name = "icrc1_decimals")]
fn icrc1_decimals() -> u8 {
    require_ledger();
    ICP_DECIMALS
}

#[ic_cdk::update(name = "icrc1_balance_of")]
fn icrc1_balance_of(account: Icrc1Account) -> Nat {
    let _ = account;
    Nat::from(STATE.with_borrow(|stored| match stored.as_ref() {
        Some(StubState::Ledger { balance_e8s, .. }) => *balance_e8s,
        _ => trap("ICP refill stub is not the Ledger role"),
    }))
}

#[ic_cdk::update(name = "icrc1_transfer")]
fn icrc1_transfer(request: TransferArg) -> Result<Nat, TransferError> {
    STATE.with_borrow_mut(|stored| {
        let Some(StubState::Ledger {
            balance_e8s,
            transfer,
        }) = stored.as_mut()
        else {
            trap("ICP refill stub is not the Ledger role");
        };
        if let Some(receipt) = transfer.as_ref() {
            if receipt.caller == msg_caller() && receipt.request == request {
                return Err(TransferError::Duplicate {
                    duplicate_of: Nat::from(BLOCK_INDEX),
                });
            }
            return Err(TransferError::GenericError {
                error_code: Nat::from(1_u8),
                message: "one-transfer fixture binding conflict".to_string(),
            });
        }
        if request.fee.as_ref() != Some(&Nat::from(LEDGER_FEE_E8S)) {
            return Err(TransferError::BadFee {
                expected_fee: Nat::from(LEDGER_FEE_E8S),
            });
        }
        let Ok(amount_e8s) = u64::try_from(request.amount.0.clone()) else {
            return Err(TransferError::InsufficientFunds {
                balance: Nat::from(*balance_e8s),
            });
        };
        let Some(debit_e8s) = amount_e8s.checked_add(LEDGER_FEE_E8S) else {
            return Err(TransferError::InsufficientFunds {
                balance: Nat::from(*balance_e8s),
            });
        };
        if debit_e8s > *balance_e8s {
            return Err(TransferError::InsufficientFunds {
                balance: Nat::from(*balance_e8s),
            });
        }
        *balance_e8s -= debit_e8s;
        *transfer = Some(TransferReceipt {
            caller: msg_caller(),
            request,
        });
        Ok(Nat::from(BLOCK_INDEX))
    })
}

#[ic_cdk::update(name = "get_icp_xdr_conversion_rate")]
fn get_icp_xdr_conversion_rate() -> IcpXdrConversionRateResponse {
    let rate = STATE.with_borrow(|stored| match stored.as_ref() {
        Some(StubState::Cmc {
            xdr_permyriad_per_icp,
            ..
        }) => *xdr_permyriad_per_icp,
        _ => trap("ICP refill stub is not the CMC role"),
    });
    IcpXdrConversionRateResponse {
        data: IcpXdrConversionRate {
            xdr_permyriad_per_icp: rate,
            timestamp_seconds: ic_cdk::api::time() / 1_000_000_000,
        },
        hash_tree: Vec::new(),
        certificate: Vec::new(),
    }
}

#[ic_cdk::update(name = "notify_top_up")]
async fn notify_top_up(request: NotifyTopUpArg) -> Result<Nat, NotifyTopUpError> {
    let (cycles, replay) = STATE.with_borrow(|stored| match stored.as_ref() {
        Some(StubState::Cmc {
            cycles_per_notify,
            notification: Some(recorded),
            ..
        }) if recorded == &request => (*cycles_per_notify, true),
        Some(StubState::Cmc {
            notification: Some(_),
            ..
        }) => trap("CMC notification binding conflict"),
        Some(StubState::Cmc {
            cycles_per_notify, ..
        }) => (*cycles_per_notify, false),
        _ => trap("ICP refill stub is not the CMC role"),
    });
    if replay {
        return Ok(Nat::from(cycles));
    }
    if request.block_index != BLOCK_INDEX {
        return Err(NotifyTopUpError::Other {
            error_code: 2,
            error_message: "unknown Ledger block".to_string(),
        });
    }
    let call = Call::unbounded_wait(Principal::management_canister(), "deposit_cycles").with_arg(
        CanisterIdRecord {
            canister_id: request.canister_id,
        },
    );
    if call.with_cycles(cycles).await.is_err() {
        return Err(NotifyTopUpError::Other {
            error_code: 3,
            error_message: "management deposit_cycles failed".to_string(),
        });
    }
    STATE.with_borrow_mut(|stored| match stored.as_mut() {
        Some(StubState::Cmc { notification, .. }) => *notification = Some(request),
        _ => trap("ICP refill stub CMC role changed"),
    });
    Ok(Nat::from(cycles))
}

fn require_ledger() {
    STATE.with_borrow(|stored| {
        if !matches!(stored.as_ref(), Some(StubState::Ledger { .. })) {
            trap("ICP refill stub is not the Ledger role");
        }
    });
}

ic_cdk::export_candid!();
