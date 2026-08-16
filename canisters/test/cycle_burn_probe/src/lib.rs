//! Module: cycle_burn_probe
//!
//! Responsibility: provide one bounded, controller-only local burn calibration.
//! Does not own: waveform scheduling, retries, funding, timers, or production state.
//! Boundary: one successful call is permitted per installation and commits its receipt atomically.

use std::cell::Cell;

use candid::CandidType;
use ic_cdk::api::{canister_cycle_balance, cycles_burn, is_controller, msg_caller};

const MAX_BURN_CYCLES: u128 = 100_000_000_000;
const MIN_RETAINED_CYCLES: u128 = 1_000_000_000_000;

thread_local! {
    static USED: Cell<bool> = const { Cell::new(false) };
}

/// Exact same-message evidence returned by the one permitted burn.
#[derive(CandidType)]
struct BurnReceipt {
    requested_cycles: u128,
    burned_cycles: u128,
    balance_before_cycles: u128,
    balance_after_burn_cycles: u128,
}

/// Bounded rejection reasons for the local calibration probe.
#[derive(CandidType)]
enum BurnProbeError {
    AccessDenied,
    AlreadyUsed,
    AmountOutOfRange { maximum_allowed_cycles: u128 },
}

/// Burn one controller-selected amount within the immutable local calibration envelope.
#[ic_cdk::update]
fn burn_once(amount_cycles: u128) -> Result<BurnReceipt, BurnProbeError> {
    if !is_controller(&msg_caller()) {
        return Err(BurnProbeError::AccessDenied);
    }
    if USED.get() {
        return Err(BurnProbeError::AlreadyUsed);
    }

    let balance_before_cycles = canister_cycle_balance();
    let available_cycles = balance_before_cycles.saturating_sub(MIN_RETAINED_CYCLES);
    let maximum_allowed_cycles = MAX_BURN_CYCLES.min(available_cycles);
    if amount_cycles == 0 || amount_cycles > maximum_allowed_cycles {
        return Err(BurnProbeError::AmountOutOfRange {
            maximum_allowed_cycles,
        });
    }

    let burned_cycles = cycles_burn(amount_cycles);
    if burned_cycles != amount_cycles {
        ic_cdk::trap("cycle burn did not match the prevalidated amount");
    }

    USED.set(true);
    Ok(BurnReceipt {
        requested_cycles: amount_cycles,
        burned_cycles,
        balance_before_cycles,
        balance_after_burn_cycles: canister_cycle_balance(),
    })
}

ic_cdk::export_candid!();
