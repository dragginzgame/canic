//! Exact Cycles Ledger boundary stub for root pool-refill PocketIC tests.

use candid::{CandidType, Deserialize, Nat, Principal};
use std::cell::RefCell;

#[derive(CandidType, Deserialize)]
struct InitArgs {
    canister_ids: Vec<Principal>,
    expected_root: Principal,
    expected_subnet: Principal,
    pending_first_index: Option<u64>,
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
    canister_ids: Vec<Principal>,
    expected_root: Principal,
    expected_subnet: Principal,
    pending_first_index: Option<usize>,
    requests: Vec<CreateCanisterArgs>,
    request_count: u64,
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
    STATE.with_borrow_mut(|state| {
        *state = Some(State {
            canister_ids: args.canister_ids,
            expected_root: args.expected_root,
            expected_subnet: args.expected_subnet,
            pending_first_index,
            requests: Vec::new(),
            request_count: 0,
        });
    });
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
        if !request_has_exact_authority(&args, state.expected_root, state.expected_subnet) {
            return Err(generic_error("creation authority mismatch"));
        }
        let index = state.requests.len();
        let Some(&canister_id) = state.canister_ids.get(index) else {
            return Err(generic_error("creation lane capacity exhausted"));
        };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_controller_and_subnet_are_required_independently() {
        let root = Principal::from_slice(&[1; 29]);
        let subnet = Principal::from_slice(&[2; 29]);
        let request = exact_request(root, subnet);
        assert!(request_has_exact_authority(&request, root, subnet));

        let wrong_root = Principal::from_slice(&[3; 29]);
        assert!(!request_has_exact_authority(&request, wrong_root, subnet));

        let wrong_subnet = Principal::from_slice(&[4; 29]);
        assert!(!request_has_exact_authority(&request, root, wrong_subnet));
    }

    #[test]
    fn incomplete_or_zero_value_requests_are_rejected() {
        let root = Principal::from_slice(&[1; 29]);
        let subnet = Principal::from_slice(&[2; 29]);

        let mut request = exact_request(root, subnet);
        request.amount = Nat::from(0_u8);
        assert!(!request_has_exact_authority(&request, root, subnet));

        let mut request = exact_request(root, subnet);
        request.created_at_time = None;
        assert!(!request_has_exact_authority(&request, root, subnet));

        let mut request = exact_request(root, subnet);
        request.creation_args = None;
        assert!(!request_has_exact_authority(&request, root, subnet));
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
