//! Exact Cycles Ledger boundary stub for root pool-refill PocketIC tests.

use candid::{CandidType, Deserialize, Nat, Principal};
use ic_cdk::call::Call;
use std::{cell::RefCell, collections::BTreeMap};

#[derive(CandidType, Deserialize)]
struct InitArgs {
    canister_ids: Vec<Principal>,
    expected_controllers_by_index: Option<Vec<Vec<Principal>>>,
    expected_root: Principal,
    expected_subnet: Principal,
    initial_balances: Option<Vec<AccountBalance>>,
    pending_first_index: Option<u64>,
    withdrawal_fee: Option<Nat>,
}

#[derive(CandidType, Deserialize)]
struct AccountBalance {
    balance: Nat,
    owner: Principal,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct Account {
    owner: Principal,
    subaccount: Option<[u8; 32]>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct WithdrawArgs {
    amount: Nat,
    created_at_time: Option<u64>,
    from_subaccount: Option<[u8; 32]>,
    to: Principal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WithdrawalRecord {
    args: WithdrawArgs,
    block_index: u64,
    caller: Principal,
}

#[derive(CandidType)]
enum WithdrawError {
    Duplicate { duplicate_of: Nat },
    GenericError { error_code: Nat, message: String },
    InsufficientFunds { balance: Nat },
    InvalidReceiver { receiver: Principal },
}

#[derive(CandidType)]
struct CanisterIdRecord {
    canister_id: Principal,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct CreateCanisterArgs {
    from_subaccount: Option<[u8; 32]>,
    created_at_time: Option<u64>,
    amount: Nat,
    creation_args: Option<CmcCreateCanisterArgs>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct CmcCreateCanisterArgs {
    settings: Option<CanisterSettings>,
    subnet_selection: Option<SubnetSelection>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct CanisterSettings {
    controllers: Option<Vec<Principal>>,
    compute_allocation: Option<Nat>,
    memory_allocation: Option<Nat>,
    freezing_threshold: Option<Nat>,
    reserved_cycles_limit: Option<Nat>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
enum SubnetSelection {
    Subnet { subnet: Principal },
}

#[derive(CandidType)]
struct CreateCanisterSuccess {
    block_id: Nat,
    canister_id: Principal,
}

#[derive(CandidType)]
enum CreateCanisterError {
    Duplicate {
        duplicate_of: Nat,
        canister_id: Option<Principal>,
    },
    GenericError {
        message: String,
        error_code: Nat,
    },
}

struct State {
    balances: BTreeMap<Principal, u128>,
    canister_ids: Vec<Principal>,
    expected_controllers_by_index: Option<Vec<Vec<Principal>>>,
    expected_root: Principal,
    expected_subnet: Principal,
    pending_first_index: Option<usize>,
    requests: Vec<CreateCanisterArgs>,
    request_count: u64,
    withdrawal_fee: u128,
    withdrawals: Vec<WithdrawalRecord>,
}

const MAX_LANES: usize = 32;

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

#[ic_cdk::init]
fn init(args: InitArgs) {
    assert!(
        !args.canister_ids.is_empty(),
        "at least one result Canister is required"
    );
    assert!(
        args.canister_ids.len() <= MAX_LANES,
        "the qualification stub supports at most {MAX_LANES} lanes"
    );
    let pending_first_index = args
        .pending_first_index
        .map(usize::try_from)
        .transpose()
        .expect("pending lane index must fit usize");
    assert!(
        pending_first_index.is_none_or(|index| index < args.canister_ids.len()),
        "pending lane index must name one configured lane"
    );
    assert!(
        args.expected_controllers_by_index
            .as_ref()
            .is_none_or(|controllers| controllers.len() == args.canister_ids.len()),
        "per-lane controller authority must cover every configured lane"
    );
    let balances = args
        .initial_balances
        .unwrap_or_default()
        .into_iter()
        .map(|entry| {
            let balance = u128::try_from(entry.balance.0)
                .expect("initial Cycles Ledger balance must fit u128");
            (entry.owner, balance)
        })
        .collect();
    let withdrawal_fee = args.withdrawal_fee.map_or(100_000_000, |fee| {
        u128::try_from(fee.0).expect("withdrawal fee must fit u128")
    });
    STATE.with_borrow_mut(|state| {
        *state = Some(State {
            balances,
            canister_ids: args.canister_ids,
            expected_controllers_by_index: args.expected_controllers_by_index,
            expected_root: args.expected_root,
            expected_subnet: args.expected_subnet,
            pending_first_index,
            requests: Vec::new(),
            request_count: 0,
            withdrawal_fee,
            withdrawals: Vec::new(),
        });
    });
}

#[ic_cdk::query]
fn icrc1_balance_of(account: Account) -> Nat {
    if account.subaccount.is_some() {
        return Nat::from(0_u8);
    }
    STATE.with_borrow(|state| {
        Nat::from(
            state
                .as_ref()
                .expect("Cycles Ledger stub is initialized")
                .balances
                .get(&account.owner)
                .copied()
                .unwrap_or_default(),
        )
    })
}

#[ic_cdk::query]
fn icrc1_fee() -> Nat {
    STATE.with_borrow(|state| {
        Nat::from(
            state
                .as_ref()
                .expect("Cycles Ledger stub is initialized")
                .withdrawal_fee,
        )
    })
}

#[ic_cdk::update]
async fn withdraw(args: WithdrawArgs) -> Result<Nat, WithdrawError> {
    let caller = ic_cdk::api::msg_caller();
    if args.from_subaccount.is_some() || args.to != caller {
        return Err(WithdrawError::InvalidReceiver { receiver: args.to });
    }
    let (amount, block_index) = STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("Cycles Ledger stub is initialized");
        if let Some(existing) = state
            .withdrawals
            .iter()
            .find(|existing| existing.caller == caller && existing.args == args)
        {
            return Err(WithdrawError::Duplicate {
                duplicate_of: Nat::from(existing.block_index),
            });
        }
        let amount =
            u128::try_from(args.amount.0.clone()).map_err(|_| WithdrawError::GenericError {
                error_code: Nat::from(1_u8),
                message: "withdrawal amount exceeds u128".to_string(),
            })?;
        let available = state.balances.get(&caller).copied().unwrap_or_default();
        let debit = amount.checked_add(state.withdrawal_fee).ok_or_else(|| {
            WithdrawError::GenericError {
                error_code: Nat::from(2_u8),
                message: "withdrawal debit overflow".to_string(),
            }
        })?;
        if available < debit {
            return Err(WithdrawError::InsufficientFunds {
                balance: Nat::from(available),
            });
        }
        let block_index = u64::try_from(state.withdrawals.len() + 1).map_err(|_| {
            WithdrawError::GenericError {
                error_code: Nat::from(3_u8),
                message: "withdrawal history exhausted".to_string(),
            }
        })?;
        state.balances.insert(caller, available - debit);
        state.withdrawals.push(WithdrawalRecord {
            args,
            block_index,
            caller,
        });
        Ok((amount, block_index))
    })?;
    Call::bounded_wait(Principal::management_canister(), "deposit_cycles")
        .with_cycles(amount)
        .with_arg(CanisterIdRecord {
            canister_id: caller,
        })
        .await
        .map_err(|error| WithdrawError::GenericError {
            error_code: Nat::from(4_u8),
            message: format!("deposit_cycles failed: {error}"),
        })?;
    Ok(Nat::from(block_index))
}

#[ic_cdk::query]
fn withdrawal_count() -> u64 {
    STATE.with_borrow(|state| {
        u64::try_from(
            state
                .as_ref()
                .expect("Cycles Ledger stub is initialized")
                .withdrawals
                .len(),
        )
        .expect("withdrawal count fits u64")
    })
}

#[ic_cdk::update]
fn create_canister(args: CreateCanisterArgs) -> Result<CreateCanisterSuccess, CreateCanisterError> {
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("Cycles Ledger stub is initialized");
        state.request_count = state.request_count.saturating_add(1);
        if let Some(index) = state.requests.iter().position(|existing| existing == &args) {
            return Err(CreateCanisterError::Duplicate {
                duplicate_of: Nat::from(index + 1),
                canister_id: Some(state.canister_ids[index]),
            });
        }
        let index = state.requests.len();
        let Some(&canister_id) = state.canister_ids.get(index) else {
            return Err(generic_error("creation lane capacity exhausted"));
        };
        let expected_controllers = state.expected_controllers_by_index.as_ref().map_or_else(
            || vec![state.expected_root],
            |controllers| controllers[index].clone(),
        );
        if !request_has_exact_authority(&args, &expected_controllers, state.expected_subnet) {
            return Err(generic_error("creation authority mismatch"));
        }
        if let Some(available) = state.balances.get_mut(&ic_cdk::api::msg_caller()) {
            let amount = u128::try_from(args.amount.0.clone())
                .map_err(|_| generic_error("creation amount exceeds u128"))?;
            if *available < amount {
                return Err(generic_error("creation balance is insufficient"));
            }
            *available -= amount;
        }
        state.requests.push(args);
        if state.pending_first_index == Some(index) {
            state.pending_first_index = None;
            return Err(CreateCanisterError::Duplicate {
                duplicate_of: Nat::from(index + 1),
                canister_id: None,
            });
        }
        Ok(CreateCanisterSuccess {
            block_id: Nat::from(index + 1),
            canister_id,
        })
    })
}

#[ic_cdk::query]
fn request_count() -> u64 {
    STATE.with_borrow(|state| {
        state
            .as_ref()
            .expect("Cycles Ledger stub is initialized")
            .request_count
    })
}

#[ic_cdk::query]
fn requested_amounts() -> Vec<Nat> {
    STATE.with_borrow(|state| {
        state
            .as_ref()
            .expect("Cycles Ledger stub is initialized")
            .requests
            .iter()
            .map(|request| request.amount.clone())
            .collect()
    })
}

fn request_has_exact_authority(
    request: &CreateCanisterArgs,
    expected_controllers: &[Principal],
    expected_subnet: Principal,
) -> bool {
    if request.from_subaccount.is_some() || request.amount == 0_u8 {
        return false;
    }
    if request
        .created_at_time
        .is_none_or(|timestamp| timestamp == 0)
    {
        return false;
    }
    let Some(creation) = &request.creation_args else {
        return false;
    };
    let Some(settings) = &creation.settings else {
        return false;
    };
    if settings.controllers.as_deref() != Some(expected_controllers) {
        return false;
    }
    creation.subnet_selection
        == Some(SubnetSelection::Subnet {
            subnet: expected_subnet,
        })
}

fn generic_error(message: &str) -> CreateCanisterError {
    CreateCanisterError::GenericError {
        message: message.to_string(),
        error_code: Nat::from(1_u8),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(CandidType)]
    struct CreateOnlyInitArgs {
        canister_ids: Vec<Principal>,
        expected_root: Principal,
        expected_subnet: Principal,
        pending_first_index: Option<u64>,
    }

    #[test]
    fn create_only_fixture_init_decodes_with_absent_withdrawal_fields() {
        let bytes = candid::encode_one(CreateOnlyInitArgs {
            canister_ids: vec![Principal::from_slice(&[1; 29])],
            expected_root: Principal::from_slice(&[2; 29]),
            expected_subnet: Principal::from_slice(&[3; 29]),
            pending_first_index: None,
        })
        .expect("encode create-only fixture init");
        let decoded: InitArgs = candid::decode_one(&bytes).expect("decode extended fixture init");
        assert!(decoded.initial_balances.is_none());
        assert!(decoded.expected_controllers_by_index.is_none());
        assert!(decoded.withdrawal_fee.is_none());
    }

    #[test]
    fn exact_controller_and_subnet_are_required_independently() {
        let root = Principal::from_slice(&[1; 29]);
        let subnet = Principal::from_slice(&[2; 29]);
        let request = exact_request(root, subnet);
        assert!(request_has_exact_authority(&request, &[root], subnet));

        let wrong_root = Principal::from_slice(&[3; 29]);
        assert!(!request_has_exact_authority(
            &request,
            &[wrong_root],
            subnet
        ));

        let wrong_subnet = Principal::from_slice(&[4; 29]);
        assert!(!request_has_exact_authority(
            &request,
            &[root],
            wrong_subnet
        ));
    }

    #[test]
    fn incomplete_or_zero_value_requests_are_rejected() {
        let root = Principal::from_slice(&[1; 29]);
        let subnet = Principal::from_slice(&[2; 29]);

        let mut request = exact_request(root, subnet);
        request.amount = Nat::from(0_u8);
        assert!(!request_has_exact_authority(&request, &[root], subnet));

        let mut request = exact_request(root, subnet);
        request.created_at_time = None;
        assert!(!request_has_exact_authority(&request, &[root], subnet));

        let mut request = exact_request(root, subnet);
        request.creation_args = None;
        assert!(!request_has_exact_authority(&request, &[root], subnet));
    }

    fn exact_request(root: Principal, subnet: Principal) -> CreateCanisterArgs {
        CreateCanisterArgs {
            from_subaccount: None,
            created_at_time: Some(1),
            amount: Nat::from(1_u8),
            creation_args: Some(CmcCreateCanisterArgs {
                settings: Some(CanisterSettings {
                    controllers: Some(vec![root]),
                    compute_allocation: None,
                    memory_allocation: None,
                    freezing_threshold: None,
                    reserved_cycles_limit: None,
                }),
                subnet_selection: Some(SubnetSelection::Subnet { subnet }),
            }),
        }
    }
}
