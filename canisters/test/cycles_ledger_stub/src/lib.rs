//! Exact Cycles Ledger boundary stub for root pool-refill PocketIC tests.

use candid::{CandidType, Deserialize, Nat, Principal};
use std::cell::RefCell;

#[derive(CandidType, Deserialize)]
struct InitArgs {
    canister_id: Principal,
    expected_root: Principal,
    expected_subnet: Principal,
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
    canister_id: Principal,
    expected_root: Principal,
    expected_subnet: Principal,
    request: Option<CreateCanisterArgs>,
    request_count: u64,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

#[ic_cdk::init]
fn init(args: InitArgs) {
    STATE.with_borrow_mut(|state| {
        *state = Some(State {
            canister_id: args.canister_id,
            expected_root: args.expected_root,
            expected_subnet: args.expected_subnet,
            request: None,
            request_count: 0,
        });
    });
}

#[ic_cdk::update]
fn create_canister(args: CreateCanisterArgs) -> Result<CreateCanisterSuccess, CreateCanisterError> {
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("Cycles Ledger stub is initialized");
        state.request_count = state.request_count.saturating_add(1);
        if let Some(existing) = &state.request {
            if existing == &args {
                return Err(CreateCanisterError::Duplicate {
                    duplicate_of: Nat::from(1_u8),
                    canister_id: Some(state.canister_id),
                });
            }
            return Err(generic_error("conflicting creation request"));
        }
        if !request_has_exact_authority(&args, state.expected_root, state.expected_subnet) {
            return Err(generic_error("creation authority mismatch"));
        }
        state.request = Some(args);
        Ok(CreateCanisterSuccess {
            block_id: Nat::from(1_u8),
            canister_id: state.canister_id,
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

fn request_has_exact_authority(
    request: &CreateCanisterArgs,
    expected_root: Principal,
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
    if settings.controllers.as_deref() != Some(&[expected_root]) {
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
