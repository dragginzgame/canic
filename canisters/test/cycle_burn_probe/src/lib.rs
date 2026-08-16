//! Module: cycle_burn_probe
//!
//! Responsibility: execute one exact, controller-driven plateau calibration.
//! Does not own: waveform scheduling, funding, timers, retries, or production state.
//! Boundary: eighteen compile-time-bound steps are the lifetime maximum per installation.

use std::cell::RefCell;

use candid::{CandidType, Principal};
use ic_cdk::api::{canister_cycle_balance, cycles_burn, is_controller, msg_caller, time};
use serde::Deserialize;

const AUTHORIZED_TOTAL_BURN_CYCLES: u128 = 3_600_000_000_000;
const EXECUTION_ALLOWANCE_CYCLES: u128 = 100_000_000_000;
const MAX_LATENESS_NS: u64 = 60_000_000_000;
const MAX_START_LEAD_NS: u64 = 300_000_000_000;
const MIN_RETAINED_CYCLES: u128 = 1_000_000_000_000;
const MIN_START_LEAD_NS: u64 = 10_000_000_000;
const STEP_BURN_CYCLES: u128 = 200_000_000_000;
const STEP_COUNT: u32 = 18;
const STEP_SPACING_NS: u64 = 100_000_000_000;

thread_local! {
    static STATE: RefCell<ProbeState> = const { RefCell::new(ProbeState::Prepared) };
}

/// Exact same-message evidence for one permitted plateau step.
#[derive(CandidType, Clone)]
struct BurnReceipt {
    balance_after_burn_cycles: u128,
    balance_before_cycles: u128,
    burned_cycles: u128,
    caller: Principal,
    executed_at_ns: u64,
    expected_at_ns: u64,
    requested_cycles: u128,
    step_index: u32,
}

/// One of the three commands admitted by the single update surface.
#[derive(CandidType, Deserialize)]
enum ProbeCommand {
    Abort,
    Start { start_at_ns: u64 },
    Step { index: u32 },
}

/// Compact externally visible run phase.
#[derive(CandidType, Clone, Copy)]
enum RunPhase {
    Aborted,
    Completed,
    Prepared,
    Running,
}

/// Recoverable bounded evidence for the current installation.
#[derive(CandidType)]
struct ProbeStatus {
    authorized_total_burn_cycles: u128,
    current_balance_cycles: u128,
    execution_allowance_cycles: u128,
    max_lateness_ns: u64,
    minimum_retained_cycles: u128,
    next_step_index: u32,
    phase: RunPhase,
    receipts: Vec<BurnReceipt>,
    start_at_ns: Option<u64>,
    step_burn_cycles: u128,
    step_count: u32,
    step_spacing_ns: u64,
    terminal_reason: Option<TerminalReason>,
    total_burned_cycles: u128,
}

/// Three composable rejection families for the bounded probe.
#[derive(CandidType)]
enum ProbeError {
    AccessDenied,
    Conflict { phase: RunPhase },
    Invalid { reason: InvalidReason },
}

/// Exact invalid-input reason retained below the public rejection family.
#[derive(CandidType)]
enum InvalidReason {
    Balance {
        available_cycles: u128,
        required_cycles: u128,
    },
    StartWindow {
        latest_ns: u64,
        now_ns: u64,
        earliest_ns: u64,
        requested_ns: u64,
    },
    StepIndex {
        expected: u32,
        received: u32,
    },
    StepTiming {
        earliest_ns: u64,
        latest_ns: u64,
        now_ns: u64,
    },
}

/// Terminal reason explaining why no later step can burn.
#[derive(CandidType, Clone, Copy)]
enum TerminalReason {
    ControllerAbort,
    LateStep,
}

enum ProbeState {
    Aborted(RunEvidence),
    Completed(RunEvidence),
    Prepared,
    Running(RunEvidence),
}

struct RunEvidence {
    next_step_index: u32,
    receipts: Vec<BurnReceipt>,
    start_at_ns: u64,
    terminal_reason: Option<TerminalReason>,
    total_burned_cycles: u128,
}

/// Dispatch the only controller-owned plateau mutation surface.
#[ic_cdk::update]
fn probe_command(command: ProbeCommand) -> Result<ProbeStatus, ProbeError> {
    require_controller()?;

    match command {
        ProbeCommand::Abort => abort(),
        ProbeCommand::Start { start_at_ns } => start(start_at_ns)?,
        ProbeCommand::Step { index } => step(index)?,
    }

    Ok(status())
}

/// Return the immutable envelope and every committed step receipt.
#[ic_cdk::query]
fn probe_status() -> Result<ProbeStatus, ProbeError> {
    require_controller()?;
    Ok(status())
}

fn require_controller() -> Result<(), ProbeError> {
    if is_controller(&msg_caller()) {
        Ok(())
    } else {
        Err(ProbeError::AccessDenied)
    }
}

fn start(start_at_ns: u64) -> Result<(), ProbeError> {
    let now_ns = time();
    let earliest_ns = now_ns.saturating_add(MIN_START_LEAD_NS);
    let latest_ns = now_ns.saturating_add(MAX_START_LEAD_NS);
    if !(earliest_ns..=latest_ns).contains(&start_at_ns) {
        return Err(ProbeError::Invalid {
            reason: InvalidReason::StartWindow {
                earliest_ns,
                latest_ns,
                now_ns,
                requested_ns: start_at_ns,
            },
        });
    }

    let available_cycles = canister_cycle_balance();
    let required_cycles =
        AUTHORIZED_TOTAL_BURN_CYCLES + MIN_RETAINED_CYCLES + EXECUTION_ALLOWANCE_CYCLES;
    if available_cycles < required_cycles {
        return Err(ProbeError::Invalid {
            reason: InvalidReason::Balance {
                available_cycles,
                required_cycles,
            },
        });
    }

    STATE.with_borrow_mut(|state| match state {
        ProbeState::Prepared => {
            *state = ProbeState::Running(RunEvidence {
                next_step_index: 0,
                receipts: Vec::with_capacity(STEP_COUNT as usize),
                start_at_ns,
                terminal_reason: None,
                total_burned_cycles: 0,
            });
            Ok(())
        }
        _ => Err(ProbeError::Conflict {
            phase: phase(state),
        }),
    })
}

fn step(index: u32) -> Result<(), ProbeError> {
    STATE.with_borrow_mut(|state| {
        if evidence(state).is_some_and(|run| index < run.next_step_index) {
            return Ok(());
        }

        let ProbeState::Running(run) = state else {
            return Err(ProbeError::Conflict {
                phase: phase(state),
            });
        };

        if index != run.next_step_index {
            return Err(ProbeError::Invalid {
                reason: InvalidReason::StepIndex {
                    expected: run.next_step_index,
                    received: index,
                },
            });
        }

        let expected_at_ns = run
            .start_at_ns
            .saturating_add(u64::from(index) * STEP_SPACING_NS);
        let latest_ns = expected_at_ns.saturating_add(MAX_LATENESS_NS);
        let now_ns = time();
        if now_ns > latest_ns {
            let mut evidence = take_running(state);
            evidence.terminal_reason = Some(TerminalReason::LateStep);
            *state = ProbeState::Aborted(evidence);
            return Ok(());
        }
        if now_ns < expected_at_ns {
            return Err(ProbeError::Invalid {
                reason: InvalidReason::StepTiming {
                    earliest_ns: expected_at_ns,
                    latest_ns,
                    now_ns,
                },
            });
        }

        let balance_before_cycles = canister_cycle_balance();
        let required_cycles = STEP_BURN_CYCLES + MIN_RETAINED_CYCLES + EXECUTION_ALLOWANCE_CYCLES;
        if balance_before_cycles < required_cycles {
            return Err(ProbeError::Invalid {
                reason: InvalidReason::Balance {
                    available_cycles: balance_before_cycles,
                    required_cycles,
                },
            });
        }

        let caller = msg_caller();
        let executed_at_ns = time();
        let burned_cycles = cycles_burn(STEP_BURN_CYCLES);
        if burned_cycles != STEP_BURN_CYCLES {
            ic_cdk::trap("cycle burn did not match the prevalidated step amount");
        }

        run.receipts.push(BurnReceipt {
            balance_after_burn_cycles: canister_cycle_balance(),
            balance_before_cycles,
            burned_cycles,
            caller,
            executed_at_ns,
            expected_at_ns,
            requested_cycles: STEP_BURN_CYCLES,
            step_index: index,
        });
        run.next_step_index += 1;
        run.total_burned_cycles += burned_cycles;
        if run.next_step_index == STEP_COUNT {
            *state = ProbeState::Completed(take_running(state));
        }
        Ok(())
    })
}

fn abort() {
    STATE.with_borrow_mut(|state| {
        if matches!(state, ProbeState::Prepared) {
            *state = ProbeState::Aborted(RunEvidence {
                next_step_index: 0,
                receipts: Vec::new(),
                start_at_ns: 0,
                terminal_reason: Some(TerminalReason::ControllerAbort),
                total_burned_cycles: 0,
            });
        } else if matches!(state, ProbeState::Running(_)) {
            let mut evidence = take_running(state);
            evidence.terminal_reason = Some(TerminalReason::ControllerAbort);
            *state = ProbeState::Aborted(evidence);
        }
    });
}

fn take_running(state: &mut ProbeState) -> RunEvidence {
    let ProbeState::Running(evidence) = std::mem::replace(state, ProbeState::Prepared) else {
        unreachable!("caller proved the probe is running");
    };
    evidence
}

fn status() -> ProbeStatus {
    STATE.with_borrow(|state| {
        let evidence = evidence(state);
        ProbeStatus {
            authorized_total_burn_cycles: AUTHORIZED_TOTAL_BURN_CYCLES,
            current_balance_cycles: canister_cycle_balance(),
            execution_allowance_cycles: EXECUTION_ALLOWANCE_CYCLES,
            max_lateness_ns: MAX_LATENESS_NS,
            minimum_retained_cycles: MIN_RETAINED_CYCLES,
            next_step_index: evidence.map_or(0, |run| run.next_step_index),
            phase: phase(state),
            receipts: evidence.map_or_else(Vec::new, |run| run.receipts.clone()),
            start_at_ns: evidence.and_then(|run| (run.start_at_ns != 0).then_some(run.start_at_ns)),
            step_burn_cycles: STEP_BURN_CYCLES,
            step_count: STEP_COUNT,
            step_spacing_ns: STEP_SPACING_NS,
            terminal_reason: evidence.and_then(|run| run.terminal_reason),
            total_burned_cycles: evidence.map_or(0, |run| run.total_burned_cycles),
        }
    })
}

const fn evidence(state: &ProbeState) -> Option<&RunEvidence> {
    match state {
        ProbeState::Aborted(evidence)
        | ProbeState::Completed(evidence)
        | ProbeState::Running(evidence) => Some(evidence),
        ProbeState::Prepared => None,
    }
}

const fn phase(state: &ProbeState) -> RunPhase {
    match state {
        ProbeState::Aborted(_) => RunPhase::Aborted,
        ProbeState::Completed(_) => RunPhase::Completed,
        ProbeState::Prepared => RunPhase::Prepared,
        ProbeState::Running(_) => RunPhase::Running,
    }
}

ic_cdk::export_candid!();
