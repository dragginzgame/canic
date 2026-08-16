//! Crate: saltz_burner
//!
//! Responsibility: execute one immutable, controller-authorized global waveform schedule.
//! Does not own: autonomous funding, Fleet roles, generic burn requests, retries, or upgrades.
//! Boundary: one timer owns exact precompiled amounts; any abort or fault stops permanently.

use std::cell::RefCell;

use candid::CandidType;
use ic_cdk::api::{canister_cycle_balance, cycles_burn, is_controller, msg_caller, time};
use ic_timers::{
    DeclarationLifetime, OnceRegistration, TimerCompletion, TimerDirective, TimerIdentity,
    TimerRunResult, TimerSchedule, initialize_runtime, register_once,
};
use serde::Deserialize;

#[expect(
    clippy::unreadable_literal,
    reason = "build-generated exact schedule values are machine authority"
)]
mod plan {
    include!(concat!(env!("OUT_DIR"), "/executable_plan.rs"));
}

const MAX_ARM_LEAD_NS: u64 = 7 * 24 * 60 * 60 * 1_000_000_000;
const MAX_LATENESS_NS: u64 = 60_000_000_000;
const MAX_RECEIPT_PAGE_SIZE: u16 = 50;
const MIN_ARM_LEAD_NS: u64 = 60_000_000_000;
const NANOS_PER_SECOND: u64 = 1_000_000_000;
const TIMER_NAME: &str = "waveform";
const TIMER_OWNER: &str = "standalone-burner";
const TIMER_SUBSYSTEM: &str = "execution";

thread_local! {
    static REGISTRATION: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) };
    static STATE: RefCell<BurnerState> = const { RefCell::new(BurnerState::Prepared) };
}

///
/// BurnerCommand
///
/// Closed mutation surface for the one embedded waveform installation.
///
#[derive(CandidType, Deserialize)]
pub enum BurnerCommand {
    Abort,

    Arm {
        authorization_digest: Vec<u8>,
        chart_start_at_ns: u64,
    },

    AuthorizeWaveform {
        authorization_digest: Vec<u8>,
    },
}

///
/// BurnerStatusRequest
///
/// Closed observation surface for summary or bounded receipt evidence.
///
#[derive(CandidType, Deserialize)]
pub enum BurnerStatusRequest {
    Receipts { limit: u16, start: u32 },

    Summary,
}

///
/// BurnerStatusResponse
///
/// Bounded response selected by one status request variant.
///
#[derive(CandidType, Deserialize)]
pub enum BurnerStatusResponse {
    Receipts(ReceiptPage),

    Summary(Box<BurnerSummary>),
}

///
/// BurnerError
///
/// Three composed public rejection families for both endpoints.
///
#[derive(CandidType, Debug, Deserialize)]
pub enum BurnerError {
    AccessDenied,

    Conflict { phase: RunPhase },

    Rejected { reason: RejectionReason },
}

///
/// RejectionReason
///
/// Typed input or runtime reason nested below the public rejection family.
///
#[derive(CandidType, Debug, Deserialize)]
pub enum RejectionReason {
    Authorization,

    Funding {
        available_cycles: u128,
        required_cycles: u128,
    },

    ReceiptPage {
        limit: u16,
        maximum_limit: u16,
        start: u32,
    },

    StartWindow {
        alignment_ns: u64,
        earliest_chart_start_ns: u64,
        latest_chart_start_ns: u64,
        requested_chart_start_ns: u64,
    },

    Timer,
}

///
/// RunPhase
///
/// Externally visible lifetime phase for this installation.
///
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum RunPhase {
    Aborted,

    Armed,

    Completed,

    Failed,

    Prepared,

    Running,
}

///
/// TerminalReason
///
/// Reason an installation stopped before normal completion.
///
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum TerminalReason {
    ControllerAbort,

    InsufficientBalance,

    LateTimer,

    PartialBurn,

    RuntimeInvariant,

    WaveformNotAuthorized,
}

///
/// BurnerSummary
///
/// Complete immutable envelope and compact current progress projection.
///
#[derive(CandidType, Deserialize)]
pub struct BurnerSummary {
    pub authorization_digest: Vec<u8>,
    pub background_cycles_per_second: u64,
    pub chart_start_at_ns: Option<u64>,
    pub chart_step_seconds: u64,
    pub control_step_seconds: u64,
    pub current_balance_cycles: u128,
    pub execution_allowance_cycles: u128,
    pub initial_funding_cycles: u128,
    pub initial_funding_step_count: u32,
    pub kernel_gain_seconds: u64,
    pub kernel_support_seconds: u64,
    pub max_burn_rate_cycles_per_second: u64,
    pub max_lateness_ns: u64,
    pub max_total_burn_cycles: u128,
    pub minimum_arm_lead_ns: u64,
    pub minimum_retained_cycles: u128,
    pub next_step_index: u32,
    pub observation_phase_lead_seconds: u64,
    pub phase: RunPhase,
    pub pre_roll_cycles: u128,
    pub pre_roll_step_count: u32,
    pub receipt_count: u32,
    pub required_cycles_to_arm: u128,
    pub run_cycles: u128,
    pub schedule_start_at_ns: Option<u64>,
    pub target_amplitude_cycles_per_second: u64,
    pub target_floor_cycles_per_second: u64,
    pub terminal_reason: Option<TerminalReason>,
    pub total_burn_cycles: u128,
    pub total_burned_cycles: u128,
    pub total_step_count: u32,
    pub waveform_authorized: bool,
    pub waveform_step_count: u32,
}

///
/// BurnReceipt
///
/// Same-message evidence for one exact scheduled amount or terminal missed deadline.
///
#[derive(CandidType, Clone, Deserialize)]
pub struct BurnReceipt {
    pub balance_after_cycles: u128,
    pub balance_before_cycles: u128,
    pub burned_cycles: u128,
    pub executed_at_ns: u64,
    pub expected_at_ns: u64,
    pub kind: ReceiptKind,
    pub requested_cycles: u128,
    pub step_index: u32,
}

///
/// ReceiptKind
///
/// Executed portion of the embedded schedule represented by one receipt.
///
#[derive(CandidType, Clone, Copy, Deserialize)]
pub enum ReceiptKind {
    PreRoll,

    Waveform,
}

///
/// ReceiptPage
///
/// Bounded chronological receipt page and continuation position.
///
#[derive(CandidType, Deserialize)]
pub struct ReceiptPage {
    pub next_start: Option<u32>,
    pub receipts: Vec<BurnReceipt>,
    pub total_receipts: u32,
}

enum BurnerState {
    Prepared,
    Run(RunEvidence),
}

struct RunEvidence {
    chart_start_at_ns: u64,
    next_step_index: u32,
    phase: RunPhase,
    receipts: Vec<BurnReceipt>,
    schedule_start_at_ns: u64,
    terminal_reason: Option<TerminalReason>,
    total_burned_cycles: u128,
    waveform_authorized: bool,
}

#[ic_cdk::init]
fn init() {
    initialize_timer().unwrap_or_else(|()| ic_cdk::trap("failed to initialize waveform timer"));
}

/// Dispatch the only controller-owned waveform mutation surface.
#[ic_cdk::update]
fn burner_command(command: BurnerCommand) -> Result<BurnerSummary, BurnerError> {
    require_controller()?;
    match command {
        BurnerCommand::Abort => abort()?,
        BurnerCommand::Arm {
            authorization_digest,
            chart_start_at_ns,
        } => arm(&authorization_digest, chart_start_at_ns)?,
        BurnerCommand::AuthorizeWaveform {
            authorization_digest,
        } => authorize_waveform(&authorization_digest)?,
    }
    Ok(summary())
}

/// Dispatch the only controller-owned waveform observation surface.
#[ic_cdk::query]
fn burner_status(request: BurnerStatusRequest) -> Result<BurnerStatusResponse, BurnerError> {
    require_controller()?;
    match request {
        BurnerStatusRequest::Receipts { limit, start } => {
            receipt_page(start, limit).map(BurnerStatusResponse::Receipts)
        }
        BurnerStatusRequest::Summary => Ok(BurnerStatusResponse::Summary(Box::new(summary()))),
    }
}

fn require_controller() -> Result<(), BurnerError> {
    if is_controller(&msg_caller()) {
        Ok(())
    } else {
        Err(BurnerError::AccessDenied)
    }
}

fn initialize_timer() -> Result<(), ()> {
    initialize_runtime().map_err(|_| ())?;
    let identity =
        TimerIdentity::try_new(TIMER_OWNER, TIMER_SUBSYSTEM, TIMER_NAME).map_err(|_| ())?;
    let registration = register_once(identity, DeclarationLifetime::Retained, |_context| async {
        run_timer_step()
    })
    .map_err(|_| ())?;
    REGISTRATION.with_borrow_mut(|slot| *slot = Some(registration));
    Ok(())
}

fn arm(authorization_digest: &[u8], chart_start_at_ns: u64) -> Result<(), BurnerError> {
    let phase = STATE.with_borrow(run_phase);
    if phase != RunPhase::Prepared {
        return Err(BurnerError::Conflict { phase });
    }
    if authorization_digest != plan::PLAN_DIGEST {
        return Err(BurnerError::Rejected {
            reason: RejectionReason::Authorization,
        });
    }

    let now_ns = time();
    let pre_roll_duration_ns = pre_roll_duration_ns();
    let observation_phase_lead_ns = observation_phase_lead_ns();
    let earliest_chart_start_ns = now_ns
        .saturating_add(MIN_ARM_LEAD_NS)
        .saturating_add(pre_roll_duration_ns)
        .saturating_add(observation_phase_lead_ns);
    let latest_chart_start_ns = now_ns
        .saturating_add(MAX_ARM_LEAD_NS)
        .saturating_add(pre_roll_duration_ns)
        .saturating_add(observation_phase_lead_ns);
    let alignment_ns = plan::CHART_STEP_SECONDS.saturating_mul(NANOS_PER_SECOND);
    if chart_start_at_ns < earliest_chart_start_ns
        || chart_start_at_ns > latest_chart_start_ns
        || !chart_start_at_ns.is_multiple_of(alignment_ns)
    {
        return Err(BurnerError::Rejected {
            reason: RejectionReason::StartWindow {
                alignment_ns,
                earliest_chart_start_ns,
                latest_chart_start_ns,
                requested_chart_start_ns: chart_start_at_ns,
            },
        });
    }

    let available_cycles = canister_cycle_balance();
    let required_cycles = required_cycles_to_arm();
    if available_cycles < required_cycles {
        return Err(BurnerError::Rejected {
            reason: RejectionReason::Funding {
                available_cycles,
                required_cycles,
            },
        });
    }

    let schedule_start_at_ns = chart_start_at_ns - pre_roll_duration_ns - observation_phase_lead_ns;
    REGISTRATION.with_borrow(|slot| {
        slot.as_ref()
            .ok_or(BurnerError::Rejected {
                reason: RejectionReason::Timer,
            })?
            .ensure_scheduled(TimerSchedule::At(schedule_start_at_ns))
            .map_err(|_| BurnerError::Rejected {
                reason: RejectionReason::Timer,
            })
    })?;
    STATE.with_borrow_mut(|state| {
        *state = BurnerState::Run(RunEvidence {
            chart_start_at_ns,
            next_step_index: 0,
            phase: RunPhase::Armed,
            receipts: Vec::with_capacity(plan::BURN_CYCLES.len()),
            schedule_start_at_ns,
            terminal_reason: None,
            total_burned_cycles: 0,
            waveform_authorized: false,
        });
    });
    Ok(())
}

fn authorize_waveform(authorization_digest: &[u8]) -> Result<(), BurnerError> {
    let (phase, next_step_index, waveform_authorized) = STATE.with_borrow(|state| match state {
        BurnerState::Prepared => (RunPhase::Prepared, 0, false),
        BurnerState::Run(run) => (run.phase, run.next_step_index, run.waveform_authorized),
    });
    if !matches!(phase, RunPhase::Armed | RunPhase::Running) {
        return Err(BurnerError::Conflict { phase });
    }
    if authorization_digest != plan::PLAN_DIGEST {
        return Err(BurnerError::Rejected {
            reason: RejectionReason::Authorization,
        });
    }
    if waveform_authorized {
        return Ok(());
    }

    let remaining_cycles = remaining_burn_cycles(next_step_index).ok_or(BurnerError::Rejected {
        reason: RejectionReason::Timer,
    })?;
    // The externally funded allowance absorbs this message's transient cycle reservation.
    let required_cycles = remaining_cycles
        .checked_add(plan::MIN_RETAINED_CYCLES)
        .ok_or(BurnerError::Rejected {
            reason: RejectionReason::Timer,
        })?;
    let available_cycles = canister_cycle_balance();
    if available_cycles < required_cycles {
        return Err(BurnerError::Rejected {
            reason: RejectionReason::Funding {
                available_cycles,
                required_cycles,
            },
        });
    }

    STATE.with_borrow_mut(|state| {
        let BurnerState::Run(run) = state else {
            unreachable!("phase check established run evidence");
        };
        run.waveform_authorized = true;
    });
    Ok(())
}

fn abort() -> Result<(), BurnerError> {
    let phase = STATE.with_borrow(run_phase);
    if matches!(
        phase,
        RunPhase::Aborted | RunPhase::Completed | RunPhase::Failed
    ) {
        return Ok(());
    }
    REGISTRATION.with_borrow(|slot| {
        slot.as_ref()
            .ok_or(BurnerError::Rejected {
                reason: RejectionReason::Timer,
            })?
            .cancel()
            .map_err(|_| BurnerError::Rejected {
                reason: RejectionReason::Timer,
            })
    })?;
    STATE.with_borrow_mut(|state| match state {
        BurnerState::Prepared => {
            *state = BurnerState::Run(RunEvidence {
                chart_start_at_ns: 0,
                next_step_index: 0,
                phase: RunPhase::Aborted,
                receipts: Vec::new(),
                schedule_start_at_ns: 0,
                terminal_reason: Some(TerminalReason::ControllerAbort),
                total_burned_cycles: 0,
                waveform_authorized: false,
            });
        }
        BurnerState::Run(run) => {
            run.phase = RunPhase::Aborted;
            run.terminal_reason = Some(TerminalReason::ControllerAbort);
        }
    });
    Ok(())
}

fn run_timer_step() -> TimerRunResult {
    STATE.with_borrow_mut(|state| {
        let BurnerState::Run(run) = state else {
            return stopped_no_work();
        };
        if !matches!(run.phase, RunPhase::Armed | RunPhase::Running) {
            return stopped_no_work();
        }

        let index = run.next_step_index;
        let Some(burn_cycles) = plan::BURN_CYCLES.get(index as usize).copied() else {
            fail_run(run, TerminalReason::RuntimeInvariant);
            return stopped_invariant_failure();
        };
        if index >= plan::PRE_ROLL_STEP_COUNT && !run.waveform_authorized {
            fail_run(run, TerminalReason::WaveformNotAuthorized);
            return stopped_invariant_failure();
        }
        let Some(expected_deadline_ns) = expected_at_ns(run.schedule_start_at_ns, index) else {
            fail_run(run, TerminalReason::RuntimeInvariant);
            return stopped_invariant_failure();
        };
        let now_ns = time();
        if now_ns < expected_deadline_ns {
            return TimerRunResult::new(
                TimerCompletion::no_work(),
                TimerDirective::ScheduleAt(expected_deadline_ns),
            );
        }
        if now_ns > expected_deadline_ns.saturating_add(MAX_LATENESS_NS) {
            fail_run(run, TerminalReason::LateTimer);
            return stopped_invariant_failure();
        }
        let Some(total_after_exact_burn) = run.total_burned_cycles.checked_add(burn_cycles) else {
            fail_run(run, TerminalReason::RuntimeInvariant);
            return stopped_invariant_failure();
        };
        if total_after_exact_burn > plan::TOTAL_BURN_CYCLES {
            fail_run(run, TerminalReason::RuntimeInvariant);
            return stopped_invariant_failure();
        }
        let Some(required_balance) = required_balance_before_burn(burn_cycles) else {
            fail_run(run, TerminalReason::RuntimeInvariant);
            return stopped_invariant_failure();
        };
        let Some(next_step_index) = run.next_step_index.checked_add(1) else {
            fail_run(run, TerminalReason::RuntimeInvariant);
            return stopped_invariant_failure();
        };
        let next_directive = if next_step_index == total_step_count() {
            TimerDirective::Stop
        } else {
            let Some(next_at_ns) = expected_at_ns(run.schedule_start_at_ns, next_step_index) else {
                fail_run(run, TerminalReason::RuntimeInvariant);
                return stopped_invariant_failure();
            };
            TimerDirective::ScheduleAt(next_at_ns)
        };

        let balance_before_cycles = canister_cycle_balance();
        if balance_before_cycles < required_balance {
            fail_run(run, TerminalReason::InsufficientBalance);
            return stopped_invariant_failure();
        }

        let burned_cycles = if burn_cycles == 0 {
            0
        } else {
            cycles_burn(burn_cycles)
        };
        run.receipts.push(BurnReceipt {
            balance_after_cycles: canister_cycle_balance(),
            balance_before_cycles,
            burned_cycles,
            executed_at_ns: time(),
            expected_at_ns: expected_deadline_ns,
            kind: if index < plan::PRE_ROLL_STEP_COUNT {
                ReceiptKind::PreRoll
            } else {
                ReceiptKind::Waveform
            },
            requested_cycles: burn_cycles,
            step_index: index,
        });
        run.next_step_index = next_step_index;
        run.total_burned_cycles += burned_cycles;
        if burned_cycles != burn_cycles {
            fail_run(run, TerminalReason::PartialBurn);
            return TimerRunResult::new(
                TimerCompletion::invariant_failure(1),
                TimerDirective::Stop,
            );
        }
        run.phase = if next_step_index == total_step_count() {
            RunPhase::Completed
        } else {
            RunPhase::Running
        };
        TimerRunResult::new(TimerCompletion::success(1), next_directive)
    })
}

const fn stopped_no_work() -> TimerRunResult {
    TimerRunResult::new(TimerCompletion::no_work(), TimerDirective::Stop)
}

const fn stopped_invariant_failure() -> TimerRunResult {
    TimerRunResult::new(TimerCompletion::invariant_failure(0), TimerDirective::Stop)
}

const fn fail_run(run: &mut RunEvidence, reason: TerminalReason) {
    run.phase = RunPhase::Failed;
    run.terminal_reason = Some(reason);
}

fn expected_at_ns(schedule_start_at_ns: u64, index: u32) -> Option<u64> {
    u64::from(index)
        .checked_mul(plan::CONTROL_STEP_SECONDS)
        .and_then(|seconds| seconds.checked_mul(NANOS_PER_SECOND))
        .and_then(|offset| schedule_start_at_ns.checked_add(offset))
}

fn pre_roll_duration_ns() -> u64 {
    u64::from(plan::PRE_ROLL_STEP_COUNT) * plan::CONTROL_STEP_SECONDS * NANOS_PER_SECOND
}

const fn observation_phase_lead_ns() -> u64 {
    plan::OBSERVATION_PHASE_LEAD_SECONDS * NANOS_PER_SECOND
}

const fn required_cycles_to_arm() -> u128 {
    plan::INITIAL_FUNDING_CYCLES + plan::MIN_RETAINED_CYCLES + plan::EXECUTION_ALLOWANCE_CYCLES
}

const fn required_balance_before_burn(burn_cycles: u128) -> Option<u128> {
    // Embedded burn allocation is separate; ordinary execution may consume the allowance.
    burn_cycles.checked_add(plan::MIN_RETAINED_CYCLES)
}

fn remaining_burn_cycles(next_step_index: u32) -> Option<u128> {
    let start = usize::try_from(next_step_index).ok()?;
    plan::BURN_CYCLES
        .get(start..)?
        .iter()
        .try_fold(0_u128, |sum, amount| sum.checked_add(*amount))
}

const fn total_step_count() -> u32 {
    plan::PRE_ROLL_STEP_COUNT + plan::WAVEFORM_STEP_COUNT
}

fn summary() -> BurnerSummary {
    STATE.with_borrow(|state| {
        let run = run_evidence(state);
        BurnerSummary {
            authorization_digest: plan::PLAN_DIGEST.to_vec(),
            background_cycles_per_second: plan::BACKGROUND_CYCLES_PER_SECOND,
            chart_start_at_ns: run.and_then(|run| nonzero(run.chart_start_at_ns)),
            chart_step_seconds: plan::CHART_STEP_SECONDS,
            control_step_seconds: plan::CONTROL_STEP_SECONDS,
            current_balance_cycles: canister_cycle_balance(),
            execution_allowance_cycles: plan::EXECUTION_ALLOWANCE_CYCLES,
            initial_funding_cycles: plan::INITIAL_FUNDING_CYCLES,
            initial_funding_step_count: plan::INITIAL_FUNDING_STEP_COUNT,
            kernel_gain_seconds: plan::KERNEL_GAIN_SECONDS,
            kernel_support_seconds: plan::KERNEL_SUPPORT_SECONDS,
            max_burn_rate_cycles_per_second: plan::MAX_BURN_RATE_CYCLES_PER_SECOND,
            max_lateness_ns: MAX_LATENESS_NS,
            max_total_burn_cycles: plan::MAX_TOTAL_BURN_CYCLES,
            minimum_arm_lead_ns: MIN_ARM_LEAD_NS,
            minimum_retained_cycles: plan::MIN_RETAINED_CYCLES,
            next_step_index: run.map_or(0, |run| run.next_step_index),
            observation_phase_lead_seconds: plan::OBSERVATION_PHASE_LEAD_SECONDS,
            phase: run_phase(state),
            pre_roll_cycles: plan::PRE_ROLL_CYCLES,
            pre_roll_step_count: plan::PRE_ROLL_STEP_COUNT,
            receipt_count: run.map_or(0, |run| {
                u32::try_from(run.receipts.len()).expect("receipt count is bounded")
            }),
            required_cycles_to_arm: required_cycles_to_arm(),
            run_cycles: plan::RUN_CYCLES,
            schedule_start_at_ns: run.and_then(|run| nonzero(run.schedule_start_at_ns)),
            target_amplitude_cycles_per_second: plan::TARGET_AMPLITUDE_CYCLES_PER_SECOND,
            target_floor_cycles_per_second: plan::TARGET_FLOOR_CYCLES_PER_SECOND,
            terminal_reason: run.and_then(|run| run.terminal_reason),
            total_burn_cycles: plan::TOTAL_BURN_CYCLES,
            total_burned_cycles: run.map_or(0, |run| run.total_burned_cycles),
            total_step_count: total_step_count(),
            waveform_authorized: run.is_some_and(|run| run.waveform_authorized),
            waveform_step_count: plan::WAVEFORM_STEP_COUNT,
        }
    })
}

fn receipt_page(start: u32, limit: u16) -> Result<ReceiptPage, BurnerError> {
    if limit == 0 || limit > MAX_RECEIPT_PAGE_SIZE {
        return Err(BurnerError::Rejected {
            reason: RejectionReason::ReceiptPage {
                limit,
                maximum_limit: MAX_RECEIPT_PAGE_SIZE,
                start,
            },
        });
    }
    STATE.with_borrow(|state| {
        let receipts = run_evidence(state).map_or(&[][..], |run| run.receipts.as_slice());
        let start_index = usize::try_from(start).unwrap_or(usize::MAX);
        if start_index > receipts.len() {
            return Err(BurnerError::Rejected {
                reason: RejectionReason::ReceiptPage {
                    limit,
                    maximum_limit: MAX_RECEIPT_PAGE_SIZE,
                    start,
                },
            });
        }
        let end = start_index
            .saturating_add(usize::from(limit))
            .min(receipts.len());
        let next_start = (end < receipts.len()).then(|| {
            u32::try_from(end).expect("receipt count is bounded by the embedded schedule")
        });
        Ok(ReceiptPage {
            next_start,
            receipts: receipts[start_index..end].to_vec(),
            total_receipts: u32::try_from(receipts.len()).expect("receipt count is bounded"),
        })
    })
}

const fn run_phase(state: &BurnerState) -> RunPhase {
    match state {
        BurnerState::Prepared => RunPhase::Prepared,
        BurnerState::Run(run) => run.phase,
    }
}

const fn run_evidence(state: &BurnerState) -> Option<&RunEvidence> {
    match state {
        BurnerState::Prepared => None,
        BurnerState::Run(run) => Some(run),
    }
}

const fn nonzero(value: u64) -> Option<u64> {
    if value == 0 { None } else { Some(value) }
}

#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    ic_cdk::trap("waveform upgrades are prohibited while this installation exists");
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    ic_cdk::trap("waveform upgrades are prohibited; reinstall a new inert installation");
}

ic_cdk::export_candid!();

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_embedded_schedule_retains_exact_timing_and_total() {
        assert_eq!(plan::BURN_CYCLES.len(), total_step_count() as usize);
        assert_eq!(
            plan::BURN_CYCLES.iter().copied().sum::<u128>(),
            plan::TOTAL_BURN_CYCLES
        );
        assert_eq!(plan::BACKGROUND_CYCLES_PER_SECOND, 30_000_000_000);
        assert_eq!(plan::KERNEL_GAIN_SECONDS, 4_201);
        assert_eq!(plan::KERNEL_SUPPORT_SECONDS, 3_600);
        assert_eq!(plan::OBSERVATION_PHASE_LEAD_SECONDS, 100);
        assert_eq!(plan::TARGET_FLOOR_CYCLES_PER_SECOND, 100_000_000_000);
        assert_eq!(plan::TARGET_AMPLITUDE_CYCLES_PER_SECOND, 50_000_000_000);
        assert_eq!(
            plan::BURN_CYCLES[..plan::PRE_ROLL_STEP_COUNT as usize]
                .iter()
                .copied()
                .sum::<u128>(),
            plan::PRE_ROLL_CYCLES
        );
        assert_eq!(
            plan::BURN_CYCLES[..plan::INITIAL_FUNDING_STEP_COUNT as usize]
                .iter()
                .copied()
                .sum::<u128>(),
            plan::INITIAL_FUNDING_CYCLES
        );
        assert_eq!(
            required_cycles_to_arm(),
            plan::INITIAL_FUNDING_CYCLES
                + plan::MIN_RETAINED_CYCLES
                + plan::EXECUTION_ALLOWANCE_CYCLES
        );
        assert_eq!(plan::INITIAL_FUNDING_CYCLES, plan::PRE_ROLL_CYCLES);
        assert_eq!(
            required_balance_before_burn(plan::BURN_CYCLES[0]),
            plan::BURN_CYCLES[0].checked_add(plan::MIN_RETAINED_CYCLES)
        );
        assert_eq!(
            plan::BURN_CYCLES[plan::PRE_ROLL_STEP_COUNT as usize..]
                .iter()
                .copied()
                .sum::<u128>(),
            plan::RUN_CYCLES
        );

        let start = 1_000_000_000_000;
        let last = expected_at_ns(start, total_step_count() - 1).expect("last deadline");
        assert_eq!(
            last,
            start
                + u64::from(total_step_count() - 1) * plan::CONTROL_STEP_SECONDS * NANOS_PER_SECOND
        );
    }
}
