//! Module: cycle_burn_probe
//!
//! Responsibility: provide one exact, controller-only cycle-burn calibration.
//! Does not own: waveform scheduling, retries, funding, timers, or production state.
//! Boundary: one successful call is permitted per installation and commits its receipt atomically.

use std::cell::RefCell;

use candid::{CandidType, Principal};
use ic_cdk::api::{canister_cycle_balance, cycles_burn, is_controller, msg_caller, time};

const AUTHORIZED_BURN_CYCLES: u128 = 4_000_000_000_000;
const MIN_RETAINED_CYCLES: u128 = 1_000_000_000_000;

thread_local! {
    static RECEIPT: RefCell<Option<BurnReceipt>> = const { RefCell::new(None) };
}

/// Exact same-message evidence returned by the one permitted burn.
#[derive(CandidType, Clone)]
struct BurnReceipt {
    requested_cycles: u128,
    burned_cycles: u128,
    balance_before_cycles: u128,
    balance_after_burn_cycles: u128,
    caller: Principal,
    executed_at_ns: u64,
}

/// Recoverable current evidence for an uncertain update response.
#[derive(CandidType)]
struct BurnStatus {
    authorized_burn_cycles: u128,
    minimum_retained_cycles: u128,
    current_balance_cycles: u128,
    receipt: Option<BurnReceipt>,
}

/// Bounded rejection reasons for the calibration probe.
#[derive(CandidType)]
enum BurnProbeError {
    AccessDenied,
    AlreadyUsed,
    InsufficientBalance {
        available_cycles: u128,
        required_cycles: u128,
    },
}

/// Burn the one compile-time-authorized calibration amount.
#[ic_cdk::update]
fn burn_once() -> Result<BurnReceipt, BurnProbeError> {
    if !is_controller(&msg_caller()) {
        return Err(BurnProbeError::AccessDenied);
    }
    if RECEIPT.with_borrow(Option::is_some) {
        return Err(BurnProbeError::AlreadyUsed);
    }

    let balance_before_cycles = canister_cycle_balance();
    let required_cycles = AUTHORIZED_BURN_CYCLES + MIN_RETAINED_CYCLES;
    if balance_before_cycles < required_cycles {
        return Err(BurnProbeError::InsufficientBalance {
            available_cycles: balance_before_cycles,
            required_cycles,
        });
    }

    let caller = msg_caller();
    let executed_at_ns = time();
    let burned_cycles = cycles_burn(AUTHORIZED_BURN_CYCLES);
    if burned_cycles != AUTHORIZED_BURN_CYCLES {
        ic_cdk::trap("cycle burn did not match the prevalidated amount");
    }

    let receipt = BurnReceipt {
        requested_cycles: AUTHORIZED_BURN_CYCLES,
        burned_cycles,
        balance_before_cycles,
        balance_after_burn_cycles: canister_cycle_balance(),
        caller,
        executed_at_ns,
    };
    RECEIPT.with_borrow_mut(|stored| *stored = Some(receipt.clone()));
    Ok(receipt)
}

/// Return the immutable authorization and committed receipt, if any.
#[ic_cdk::query]
fn burn_status() -> Result<BurnStatus, BurnProbeError> {
    if !is_controller(&msg_caller()) {
        return Err(BurnProbeError::AccessDenied);
    }

    Ok(BurnStatus {
        authorized_burn_cycles: AUTHORIZED_BURN_CYCLES,
        minimum_retained_cycles: MIN_RETAINED_CYCLES,
        current_balance_cycles: canister_cycle_balance(),
        receipt: RECEIPT.with_borrow(Clone::clone),
    })
}

ic_cdk::export_candid!();
