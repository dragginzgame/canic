//! Module: workflow::runtime::cycles
//!
//! Responsibility: record cycle observations and run configured automatic funding.
//! Does not own: funding policy, stable telemetry schemas, or timer arbitration.
//! Boundary: lifecycle and funding events record history; one timer owns top-up safety.

pub mod query;

use crate::{
    InternalError,
    cdk::types::Cycles,
    config::schema::TopupPolicy,
    diagnostics::codes,
    domain::{
        icp_refill::{IcpRefillStatus, IcpRefillTrigger, icp_refill_outcome_is_resumable},
        policy::pure as policy,
    },
    dto::fleet_funding::{
        FleetRootFundingNoGrantReason, FleetRootFundingNoGrantReceipt, FleetRootFundingRequest,
        FleetRootFundingResponse,
    },
    dto::icp_refill::IcpRefillResponse,
    dto::rpc::{CyclesFundingPreflightResponse, CyclesResponse},
    log,
    log::Topic,
    model::replay::OperationId,
    ops::{
        config::ConfigOps,
        ic::IcOps,
        rpc::request::RequestOps,
        runtime::{
            env::EnvOps,
            metrics::cycles_topup::CyclesTopupMetrics,
            root_funding::{RootFundingRuntimeApi, RootFundingRuntimeConfig},
        },
        storage::async_job_recovery::AsyncJobOwner,
        storage::{
            cycles::{CycleTopupEventOps, CycleTrackerOps},
            state::fleet::FleetStateOps,
        },
    },
    workflow::ic::icp_refill::IcpRefillWorkflow,
    workflow::runtime::{
        async_job::AsyncJobWorkflow,
        timer::{
            TimerAuthorityWorkflow, TimerError, require_active, retain_owned_once, with_owned_once,
        },
    },
};
use ic_timers::{
    DeclarationLifetime, OnceContext, OnceRegistration, TimerCompletion, TimerDirective,
    TimerIdentity, TimerRunResult, TimerSchedule, register_once,
};
use std::{
    cell::{Cell, RefCell},
    time::Duration,
};

const NANOS_PER_SECOND: u64 = 1_000_000_000;
const RETENTION_BATCH_SIZE: usize = 128;
const RETRY_INITIAL: Duration = Duration::from_mins(1);
const RETRY_MAX: Duration = Duration::from_mins(30);

thread_local! {
    static RESOURCE_EXHAUSTION_RECOVERY_CONSUMED: Cell<bool> = const { Cell::new(false) };
    static TOPUP_TIMER: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) };
}

struct AutomaticTopupConfig {
    threshold: u128,
    minimum_funding_spacing_secs: u64,
    new_requests_enabled: bool,
    target: AutomaticTopupTarget,
}

enum AutomaticTopupTarget {
    Parent { amount: Cycles },
    RootCoordinator,
}

struct CycleBalanceSample {
    timestamp_secs: u64,
    cycles: Cycles,
}

enum ParentFundingOutcome {
    AutomaticRefill(IcpRefillResponse),
    PreflightRejected(CyclesFundingPreflightResponse),
    RootNoGrant(Box<FleetRootFundingNoGrantReceipt>),
    Transferred,
}

/// Runtime owner for cycle observations and configured automatic funding.
pub struct CycleWorkflow;

impl CycleWorkflow {
    /// Return the exact automatic-top-up native identity.
    pub(crate) fn timer_identity() -> Result<TimerIdentity, TimerError> {
        TimerIdentity::try_new("canic", "cycles", "topup").map_err(Into::into)
    }

    pub(crate) fn claimed_timer_identity() -> Result<Option<TimerIdentity>, TimerError> {
        with_owned_once(&TOPUP_TIMER, |registration| registration.identity().clone())
    }

    pub(crate) fn cancel_timer() -> Result<(), TimerError> {
        if let Some(result) = with_owned_once(&TOPUP_TIMER, OnceRegistration::cancel)? {
            result?;
        }
        Ok(())
    }

    /// Recover one expired automatic-top-up attempt from current capability demand.
    pub(crate) fn recover_expired_timer(now_ns: u64) -> bool {
        let owner = AsyncJobOwner::CycleTopup;
        if !AsyncJobWorkflow::has_expired_attempt(owner, now_ns) {
            return false;
        }
        match Self::automatic_topup_config() {
            Ok(Some(_)) => {}
            Ok(None) => return AsyncJobWorkflow::abandon_expired(owner, now_ns),
            Err(_) => return false,
        }
        let Some(attempt) = AsyncJobWorkflow::claim_expired(owner, now_ns) else {
            return false;
        };
        ic_cdk::futures::spawn(async move {
            let result = Self::run_attempt(attempt).await;
            let _ = AsyncJobWorkflow::finish(attempt, result);
        });
        true
    }

    /// Record the lifecycle balance and reconcile the sole top-up safety deadline.
    pub fn start() -> Result<(), InternalError> {
        let config = startup_phase("configuration", Self::automatic_topup_config())?;
        let current_request = startup_phase(
            "request_recovery",
            Self::current_root_request(config.as_ref()),
        )?;
        let previous = Self::latest_observation();
        let sample = Self::read_sample();
        Self::record_observation(&sample);

        if config.is_none() && !EnvOps::is_root() {
            CyclesTopupMetrics::record_policy_missing();
        }
        startup_phase(
            "timer_reconciliation",
            Self::reconcile_from_sample(
                config.as_ref(),
                current_request.as_ref(),
                &sample,
                previous,
            ),
        )
    }

    fn reconcile_from_sample(
        config: Option<&AutomaticTopupConfig>,
        current_request: Option<&FleetRootFundingRequest>,
        sample: &CycleBalanceSample,
        previous: Option<policy::cycles::CycleBalanceObservation>,
    ) -> Result<(), InternalError> {
        let deadline = config
            .and_then(|config| {
                if current_request.is_some() {
                    return Some(Ok(IcOps::now_nanos()));
                }
                if !config.new_requests_enabled {
                    return None;
                }
                Some(Self::deadline_ns(
                    IcOps::now_nanos(),
                    policy::cycles::cycle_topup_timing(
                        sample.timestamp_secs,
                        sample.cycles.to_u128(),
                        config.threshold,
                        previous,
                    ),
                ))
            })
            .transpose()?;
        Self::reconcile_timer(deadline)?;
        Ok(())
    }

    async fn run_registered() -> TimerRunResult {
        let attempt = match AsyncJobWorkflow::claim(AsyncJobOwner::CycleTopup) {
            Ok(attempt) => attempt,
            Err(result) => return result,
        };
        let result = Self::run_attempt(attempt).await;
        AsyncJobWorkflow::finish(attempt, result)
    }

    async fn run_attempt(
        attempt: crate::ops::storage::async_job_recovery::AsyncJobAttempt,
    ) -> TimerRunResult {
        let Some(operation_id) = attempt.operation_id() else {
            return TimerRunResult::new(
                TimerCompletion::invariant_failure(0),
                TimerDirective::Stop,
            );
        };
        Self::run_topup(operation_id).await
    }

    async fn run_topup(operation_id: OperationId) -> TimerRunResult {
        let config = match Self::automatic_topup_config() {
            Ok(Some(config)) => config,
            Ok(None) => {
                return TimerRunResult::new(TimerCompletion::no_work(), TimerDirective::Stop);
            }
            Err(err) => {
                CyclesTopupMetrics::record_config_error();
                log!(Topic::Cycles, Error, "automatic top-up stopped: {err}");
                return TimerRunResult::new(
                    TimerCompletion::invariant_failure(0),
                    TimerDirective::Stop,
                );
            }
        };
        if matches!(config.target, AutomaticTopupTarget::RootCoordinator) {
            match IcpRefillWorkflow::active_refill_trigger() {
                Ok(Some(IcpRefillTrigger::Automatic { .. })) => {
                    return Self::resume_automatic_refill(&config).await;
                }
                Ok(Some(IcpRefillTrigger::Manual)) => {
                    log!(
                        Topic::Cycles,
                        Warn,
                        "automatic Root top-up is waiting for the active manual ICP refill"
                    );
                    return retryable_topup_after(Duration::from_secs(
                        config.minimum_funding_spacing_secs.max(1),
                    ));
                }
                Ok(None) => {}
                Err(err) => return Self::finish_funding_failure(err),
            }
        }
        let current_request = match Self::current_root_request(Some(&config)) {
            Ok(current) => current,
            Err(err) => {
                log!(Topic::Cycles, Error, "automatic top-up stopped: {err}");
                return TimerRunResult::new(
                    TimerCompletion::invariant_failure(0),
                    TimerDirective::Stop,
                );
            }
        };
        let previous = Self::latest_observation();
        let sample = Self::read_sample();
        Self::record_observation(&sample);
        let timing = policy::cycles::cycle_topup_timing(
            sample.timestamp_secs,
            sample.cycles.to_u128(),
            config.threshold,
            previous,
        );

        match select_topup_demand(
            current_request.is_some(),
            config.new_requests_enabled,
            timing,
        ) {
            AutomaticTopupDemand::Stop => {
                return TimerRunResult::new(TimerCompletion::no_work(), TimerDirective::Stop);
            }
            AutomaticTopupDemand::Wait(timing) => {
                CyclesTopupMetrics::record_above_threshold();
                return match Self::directive(IcOps::now_nanos(), timing) {
                    Ok(directive) => TimerRunResult::new(TimerCompletion::no_work(), directive),
                    Err(err) => {
                        log!(Topic::Cycles, Error, "automatic top-up stopped: {err}");
                        TimerRunResult::new(
                            TimerCompletion::invariant_failure(0),
                            TimerDirective::Stop,
                        )
                    }
                };
            }
            AutomaticTopupDemand::Create | AutomaticTopupDemand::Resume => {}
        }

        let result = match &config.target {
            AutomaticTopupTarget::Parent { amount } => {
                Self::request_parent_funding(amount, operation_id).await
            }
            AutomaticTopupTarget::RootCoordinator => {
                Self::request_root_funding(current_request).await
            }
        };
        let after_parent = Self::read_sample();
        Self::record_observation(&after_parent);
        let result = Self::maybe_run_automatic_refill(result, &after_parent).await;
        let after = Self::read_sample();
        Self::record_observation(&after);

        Self::finish_topup(&config, &sample, &after, result)
    }

    async fn resume_automatic_refill(config: &AutomaticTopupConfig) -> TimerRunResult {
        let before = Self::read_sample();
        Self::record_observation(&before);
        let result = IcpRefillWorkflow::resume_automatic_refill().await;
        let after = Self::read_sample();
        Self::record_observation(&after);
        Self::finish_automatic_refill(config, &before, &after, result)
    }

    async fn maybe_run_automatic_refill(
        result: Result<ParentFundingOutcome, InternalError>,
        sample: &CycleBalanceSample,
    ) -> Result<ParentFundingOutcome, InternalError> {
        let Ok(ParentFundingOutcome::RootNoGrant(receipt)) = result else {
            return result;
        };
        if !permits_automatic_refill(receipt.reason) {
            return Ok(ParentFundingOutcome::RootNoGrant(receipt));
        }
        let Some(emergency_threshold) = IcpRefillWorkflow::automatic_refill_threshold()? else {
            return Ok(ParentFundingOutcome::RootNoGrant(receipt));
        };
        if sample.cycles.to_u128() > emergency_threshold {
            return Ok(ParentFundingOutcome::RootNoGrant(receipt));
        }

        log!(
            Topic::Cycles,
            Warn,
            "automatic Root top-up is entering protected ICP fallback after terminal Coordinator no-grant ({:?})",
            receipt.reason
        );
        IcpRefillWorkflow::start_automatic_refill(
            sample.cycles.to_u128(),
            receipt.request.operation_id,
        )
        .await
        .map(ParentFundingOutcome::AutomaticRefill)
    }

    fn finish_topup(
        config: &AutomaticTopupConfig,
        before: &CycleBalanceSample,
        after: &CycleBalanceSample,
        result: Result<ParentFundingOutcome, InternalError>,
    ) -> TimerRunResult {
        match result {
            Ok(ParentFundingOutcome::AutomaticRefill(response)) => {
                Self::finish_automatic_refill(config, before, after, Ok(response))
            }
            Ok(ParentFundingOutcome::Transferred) => {
                Self::finish_transferred_topup(config, before, after)
            }
            Ok(ParentFundingOutcome::PreflightRejected(preflight)) => {
                Self::finish_preflight_rejection(preflight)
            }
            Ok(ParentFundingOutcome::RootNoGrant(receipt)) => {
                Self::finish_root_no_grant(config, receipt.reason)
            }
            Err(failure) => Self::finish_funding_failure(failure),
        }
    }

    fn finish_automatic_refill(
        config: &AutomaticTopupConfig,
        before: &CycleBalanceSample,
        after: &CycleBalanceSample,
        result: Result<IcpRefillResponse, InternalError>,
    ) -> TimerRunResult {
        match result {
            Ok(response) if response.status == IcpRefillStatus::Completed => {
                log!(
                    Topic::Cycles,
                    Ok,
                    "automatic Root ICP refill completed operation_id={} cycles_sent={:?}",
                    OperationId::from_bytes(response.operation_id),
                    response.cycles_sent
                );
                Self::finish_transferred_topup(config, before, after)
            }
            Ok(response)
                if icp_refill_outcome_is_resumable(
                    response.status,
                    response.error_code,
                    response.ledger_block_index.is_some(),
                ) =>
            {
                log!(
                    Topic::Cycles,
                    Warn,
                    "automatic Root ICP refill remains resumable operation_id={} status={:?} error={:?}",
                    OperationId::from_bytes(response.operation_id),
                    response.status,
                    response.error_code
                );
                retryable_topup_after(RETRY_INITIAL)
            }
            Ok(response) => {
                log!(
                    Topic::Cycles,
                    Error,
                    "automatic Root ICP refill stopped with terminal outcome operation_id={} status={:?} error={:?}",
                    OperationId::from_bytes(response.operation_id),
                    response.status,
                    response.error_code
                );
                TimerRunResult::new(TimerCompletion::invariant_failure(0), TimerDirective::Stop)
            }
            Err(err) => Self::finish_automatic_refill_failure(err),
        }
    }

    fn finish_automatic_refill_failure(failure: InternalError) -> TimerRunResult {
        if automatic_refill_failure_disposition(&failure)
            == AutomaticRefillFailureDisposition::BackOff
        {
            log!(
                Topic::Cycles,
                Warn,
                "automatic Root ICP refill is blocked by the current conversion rate and will retry: {}",
                failure
            );
            let streak = Self::consecutive_expected_failures();
            return retryable_topup_after(retry_delay(streak));
        }
        Self::finish_funding_failure(failure)
    }

    fn finish_transferred_topup(
        config: &AutomaticTopupConfig,
        before: &CycleBalanceSample,
        after: &CycleBalanceSample,
    ) -> TimerRunResult {
        reset_resource_exhaustion_recovery();
        if !config.new_requests_enabled {
            return TimerRunResult::new(TimerCompletion::success(1), TimerDirective::Stop);
        }
        let timing = policy::cycles::cycle_topup_timing(
            after.timestamp_secs,
            after.cycles.to_u128(),
            config.threshold,
            Some(policy::cycles::CycleBalanceObservation {
                timestamp_secs: before.timestamp_secs,
                balance: before.cycles.to_u128(),
            }),
        );
        let directive = if matches!(timing, policy::cycles::CycleTopupTiming::Due) {
            Self::deadline_after_secs(IcOps::now_nanos(), config.minimum_funding_spacing_secs)
                .map(TimerDirective::ScheduleAt)
        } else {
            Self::directive(IcOps::now_nanos(), timing)
        };
        match directive {
            Ok(directive) => TimerRunResult::new(TimerCompletion::success(1), directive),
            Err(err) => {
                log!(Topic::Cycles, Error, "automatic top-up stopped: {err}");
                TimerRunResult::new(TimerCompletion::invariant_failure(0), TimerDirective::Stop)
            }
        }
    }

    fn finish_preflight_rejection(preflight: CyclesFundingPreflightResponse) -> TimerRunResult {
        match preflight {
            CyclesFundingPreflightResponse::CooldownActive { retry_after_secs } => {
                log!(
                    Topic::Cycles,
                    Warn,
                    "automatic top-up is waiting for the parent funding cooldown ({retry_after_secs}s)"
                );
                retryable_topup_after(Duration::from_secs(retry_after_secs.max(1)))
            }
            CyclesFundingPreflightResponse::ParentFundingUnavailable { approved_cycles } => {
                log!(
                    Topic::Cycles,
                    Warn,
                    "automatic top-up is waiting for parent funding capacity (approved_cycles={approved_cycles})"
                );
                let streak = Self::consecutive_expected_failures();
                retryable_topup_after(retry_delay(streak))
            }
            CyclesFundingPreflightResponse::ChildBudgetExhausted {
                remaining_child_budget,
                max_per_child,
            } => {
                log!(
                    Topic::Cycles,
                    Warn,
                    "automatic top-up stopped at the parent child-budget limit (remaining_child_budget={remaining_child_budget}, max_per_child={max_per_child})"
                );
                TimerRunResult::new(TimerCompletion::no_work(), TimerDirective::Stop)
            }
        }
    }

    fn finish_root_no_grant(
        config: &AutomaticTopupConfig,
        reason: FleetRootFundingNoGrantReason,
    ) -> TimerRunResult {
        if !config.new_requests_enabled {
            return TimerRunResult::new(TimerCompletion::no_work(), TimerDirective::Stop);
        }
        match classify_root_no_grant(reason) {
            RootNoGrantDisposition::Wait => {
                log!(
                    Topic::Cycles,
                    Warn,
                    "automatic Root top-up received no grant ({reason:?})"
                );
                retryable_topup_after(Duration::from_secs(
                    config.minimum_funding_spacing_secs.max(1),
                ))
            }
            RootNoGrantDisposition::Stop => {
                log!(
                    Topic::Cycles,
                    Error,
                    "automatic Root top-up stopped after a terminal rejection ({reason:?})"
                );
                TimerRunResult::new(TimerCompletion::invariant_failure(0), TimerDirective::Stop)
            }
        }
    }

    fn finish_funding_failure(failure: InternalError) -> TimerRunResult {
        if is_retryable_funding_error(&failure) {
            log!(
                Topic::Cycles,
                Warn,
                "automatic top-up will retry: {}",
                failure
            );
            let streak = Self::consecutive_expected_failures();
            return retryable_topup_after(retry_delay(streak));
        }
        if claim_resource_exhaustion_recovery(&failure) {
            log!(
                Topic::Cycles,
                Warn,
                "automatic top-up will make one resource-exhaustion recovery attempt: {}",
                failure
            );
            return retryable_topup_after(RETRY_INITIAL);
        }
        log!(
            Topic::Cycles,
            Error,
            "automatic top-up stopped: {}",
            failure
        );
        TimerRunResult::new(TimerCompletion::invariant_failure(0), TimerDirective::Stop)
    }

    async fn request_parent_funding(
        amount: &Cycles,
        operation_id: OperationId,
    ) -> Result<ParentFundingOutcome, InternalError> {
        CyclesTopupMetrics::record_request_scheduled();
        CycleTopupEventOps::record_scheduled(IcOps::now_secs(), amount.clone());
        match RequestOps::request_cycles_with_operation_id(amount.to_u128(), operation_id).await {
            Ok(CyclesResponse::Transferred { cycles_transferred }) => {
                let transferred = Cycles::from(cycles_transferred);
                CyclesTopupMetrics::record_request_ok();
                CycleTopupEventOps::record_ok(
                    IcOps::now_secs(),
                    amount.clone(),
                    transferred.clone(),
                );
                log!(
                    Topic::Cycles,
                    Ok,
                    "requested {amount}, topped up by {transferred}, now {}",
                    IcOps::canister_cycle_balance()
                );
                Ok(ParentFundingOutcome::Transferred)
            }
            Ok(CyclesResponse::PreflightRejected(preflight)) => {
                CyclesTopupMetrics::record_request_err();
                CycleTopupEventOps::record_err(
                    IcOps::now_secs(),
                    amount.clone(),
                    format!("parent funding preflight rejected: {preflight:?}"),
                );
                Ok(ParentFundingOutcome::PreflightRejected(preflight))
            }
            Err(err) => {
                CyclesTopupMetrics::record_request_err();
                CycleTopupEventOps::record_err(IcOps::now_secs(), amount.clone(), err.to_string());
                Err(err)
            }
        }
    }

    async fn request_root_funding(
        current_request: Option<FleetRootFundingRequest>,
    ) -> Result<ParentFundingOutcome, InternalError> {
        let request = match current_request {
            Some(request) => request,
            None => RootFundingRuntimeApi::prepare_request()?,
        };
        let amount = request.requested_cycles.clone();
        CyclesTopupMetrics::record_request_scheduled();
        CycleTopupEventOps::record_scheduled(IcOps::now_secs(), amount.clone());
        let response = match RootFundingRuntimeApi::request(request).await {
            Ok(response) => response,
            Err(err) => {
                CyclesTopupMetrics::record_request_err();
                CycleTopupEventOps::record_err(IcOps::now_secs(), amount, err.to_string());
                return Err(err);
            }
        };
        let response = RootFundingRuntimeApi::record_response(response)?;
        match response {
            FleetRootFundingResponse::Granted(receipt) => {
                let transferred = receipt.request.granted_cycles;
                CyclesTopupMetrics::record_request_ok();
                CycleTopupEventOps::record_ok(IcOps::now_secs(), amount, transferred.clone());
                log!(
                    Topic::Cycles,
                    Ok,
                    "Coordinator topped up Root by {transferred}, now {}",
                    IcOps::canister_cycle_balance()
                );
                Ok(ParentFundingOutcome::Transferred)
            }
            FleetRootFundingResponse::NoGrant(receipt) => {
                CyclesTopupMetrics::record_request_err();
                CycleTopupEventOps::record_err(
                    IcOps::now_secs(),
                    amount,
                    format!("Coordinator returned no grant: {:?}", receipt.reason),
                );
                Ok(ParentFundingOutcome::RootNoGrant(Box::new(receipt)))
            }
        }
    }

    fn automatic_topup_config() -> Result<Option<AutomaticTopupConfig>, InternalError> {
        if EnvOps::is_root() {
            let config = RootFundingRuntimeApi::config()?;
            return Ok(Some(select_root_topup_config(
                config,
                FleetStateOps::cycles_funding_enabled(),
            )));
        }
        let canister = ConfigOps::current_canister()?;
        Ok(select_automatic_topup_config(
            canister.topup,
            canister.cycles_funding.cooldown_secs,
        ))
    }

    fn current_root_request(
        config: Option<&AutomaticTopupConfig>,
    ) -> Result<Option<FleetRootFundingRequest>, InternalError> {
        match config.map(|config| &config.target) {
            Some(AutomaticTopupTarget::RootCoordinator) => RootFundingRuntimeApi::current_request(),
            _ => Ok(None),
        }
    }

    fn reconcile_timer(deadline_ns: Option<u64>) -> Result<(), TimerError> {
        require_active()?;
        if deadline_ns.is_some() {
            Self::declare_timer()?;
        }
        if !EnvOps::is_root()
            && (deadline_ns.is_some()
                || AsyncJobWorkflow::has_active_attempt(AsyncJobOwner::CycleTopup))
        {
            TimerAuthorityWorkflow::ensure_async_job_recovery_watchdog_with_automatic_topup()?;
        }
        if let Some(result) = with_owned_once(&TOPUP_TIMER, |registration| {
            registration.reconcile_schedule(deadline_ns.map(TimerSchedule::At))
        })? {
            result?;
        }
        Ok(())
    }

    fn declare_timer() -> Result<(), TimerError> {
        require_active()?;
        if with_owned_once(&TOPUP_TIMER, |_| ())?.is_some() {
            return Ok(());
        }
        let registration = register_once(
            Self::timer_identity()?,
            DeclarationLifetime::Retained,
            |_context: OnceContext| async { Self::run_registered().await },
        )?;
        retain_owned_once(&TOPUP_TIMER, registration)
    }

    fn consecutive_expected_failures() -> u64 {
        Self::timer_identity()
            .and_then(|identity| Ok(ic_timers::consecutive_expected_failures(&identity)?))
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    fn read_sample() -> CycleBalanceSample {
        CycleBalanceSample {
            timestamp_secs: IcOps::now_secs(),
            cycles: IcOps::canister_cycle_balance(),
        }
    }

    fn latest_observation() -> Option<policy::cycles::CycleBalanceObservation> {
        CycleTrackerOps::latest().map(|(timestamp_secs, cycles)| {
            policy::cycles::CycleBalanceObservation {
                timestamp_secs,
                balance: cycles.to_u128(),
            }
        })
    }

    fn record_observation(sample: &CycleBalanceSample) {
        CycleTrackerOps::record(sample.timestamp_secs, sample.cycles.clone());
        Self::purge_history(sample.timestamp_secs);
    }

    fn purge_history(now_secs: u64) {
        let cutoff = policy::cycles::retention_cutoff(now_secs);
        let purged_tracker = CycleTrackerOps::purge_before(cutoff, RETENTION_BATCH_SIZE);
        let purged_topups = CycleTopupEventOps::purge_before(cutoff, RETENTION_BATCH_SIZE);
        if purged_tracker > 0 || purged_topups > 0 {
            log!(
                Topic::Cycles,
                Info,
                "cycle history: purged {purged_tracker} balance entries and {purged_topups} top-up events"
            );
        }
    }

    fn deadline_ns(
        now_ns: u64,
        timing: policy::cycles::CycleTopupTiming,
    ) -> Result<u64, InternalError> {
        match timing {
            policy::cycles::CycleTopupTiming::Due => Ok(now_ns),
            policy::cycles::CycleTopupTiming::CheckAfter { delay_secs } => {
                Self::deadline_after_secs(now_ns, delay_secs)
            }
        }
    }

    fn directive(
        now_ns: u64,
        timing: policy::cycles::CycleTopupTiming,
    ) -> Result<TimerDirective, InternalError> {
        match timing {
            policy::cycles::CycleTopupTiming::Due => {
                Self::deadline_after_secs(now_ns, policy::cycles::CYCLE_TOPUP_MIN_CHECK_SECS)
                    .map(TimerDirective::ScheduleAt)
            }
            policy::cycles::CycleTopupTiming::CheckAfter { .. } => {
                Self::deadline_ns(now_ns, timing).map(TimerDirective::ScheduleAt)
            }
        }
    }

    fn deadline_after_secs(now_ns: u64, delay_secs: u64) -> Result<u64, InternalError> {
        let delay_ns = delay_secs
            .checked_mul(NANOS_PER_SECOND)
            .ok_or_else(InternalError::invariant)?;
        now_ns
            .checked_add(delay_ns)
            .ok_or_else(InternalError::invariant)
    }
}

fn startup_phase<T>(phase: &str, result: Result<T, InternalError>) -> Result<T, InternalError> {
    if EnvOps::is_root()
        && let Err(error) = &result
    {
        let message = format!("Root cycles runtime startup failed phase={phase} error={error}");
        IcOps::println(&message);
    }
    result
}

fn select_automatic_topup_config(
    topup: Option<TopupPolicy>,
    funding_cooldown_secs: u64,
) -> Option<AutomaticTopupConfig> {
    let topup = topup?;
    Some(AutomaticTopupConfig {
        threshold: topup.threshold.to_u128(),
        minimum_funding_spacing_secs: funding_cooldown_secs
            .max(policy::cycles::CYCLE_TOPUP_MIN_CHECK_SECS),
        new_requests_enabled: true,
        target: AutomaticTopupTarget::Parent {
            amount: topup.amount,
        },
    })
}

fn select_root_topup_config(
    config: RootFundingRuntimeConfig,
    new_requests_enabled: bool,
) -> AutomaticTopupConfig {
    AutomaticTopupConfig {
        threshold: config.request_threshold,
        minimum_funding_spacing_secs: config
            .cooldown_secs
            .max(policy::cycles::CYCLE_TOPUP_MIN_CHECK_SECS),
        new_requests_enabled,
        target: AutomaticTopupTarget::RootCoordinator,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RootNoGrantDisposition {
    Stop,
    Wait,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AutomaticRefillFailureDisposition {
    BackOff,
    StopOrRecover,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AutomaticTopupDemand {
    Create,
    Resume,
    Stop,
    Wait(policy::cycles::CycleTopupTiming),
}

const fn select_topup_demand(
    has_current_request: bool,
    new_requests_enabled: bool,
    timing: policy::cycles::CycleTopupTiming,
) -> AutomaticTopupDemand {
    if has_current_request {
        return AutomaticTopupDemand::Resume;
    }
    if !new_requests_enabled {
        return AutomaticTopupDemand::Stop;
    }
    match timing {
        policy::cycles::CycleTopupTiming::Due => AutomaticTopupDemand::Create,
        policy::cycles::CycleTopupTiming::CheckAfter { .. } => AutomaticTopupDemand::Wait(timing),
    }
}

const fn classify_root_no_grant(reason: FleetRootFundingNoGrantReason) -> RootNoGrantDisposition {
    match reason {
        FleetRootFundingNoGrantReason::CooldownActive
        | FleetRootFundingNoGrantReason::CoordinatorReserveUnavailable
        | FleetRootFundingNoGrantReason::FleetWindowExhausted
        | FleetRootFundingNoGrantReason::RootWindowExhausted => RootNoGrantDisposition::Wait,
        FleetRootFundingNoGrantReason::InvalidRequest
        | FleetRootFundingNoGrantReason::FleetAutomaticCapExhausted
        | FleetRootFundingNoGrantReason::FundingDisabled
        | FleetRootFundingNoGrantReason::PolicyMismatch
        | FleetRootFundingNoGrantReason::RegistryStale
        | FleetRootFundingNoGrantReason::RootAutomaticCapExhausted
        | FleetRootFundingNoGrantReason::RootIneligible
        | FleetRootFundingNoGrantReason::RootRejected => RootNoGrantDisposition::Stop,
    }
}

fn automatic_refill_failure_disposition(
    error: &InternalError,
) -> AutomaticRefillFailureDisposition {
    if error.code() == codes::CAPACITY_INVALID_STATE {
        AutomaticRefillFailureDisposition::BackOff
    } else {
        AutomaticRefillFailureDisposition::StopOrRecover
    }
}

const fn permits_automatic_refill(reason: FleetRootFundingNoGrantReason) -> bool {
    let policy_reason = match reason {
        FleetRootFundingNoGrantReason::CoordinatorReserveUnavailable => {
            Some(policy::fleet_funding::FleetRootGrantNoGrantReason::CoordinatorReserveUnavailable)
        }
        FleetRootFundingNoGrantReason::FleetWindowExhausted => {
            Some(policy::fleet_funding::FleetRootGrantNoGrantReason::FleetWindowExhausted)
        }
        FleetRootFundingNoGrantReason::RootWindowExhausted => {
            Some(policy::fleet_funding::FleetRootGrantNoGrantReason::RootWindowExhausted)
        }
        FleetRootFundingNoGrantReason::CooldownActive
        | FleetRootFundingNoGrantReason::FleetAutomaticCapExhausted
        | FleetRootFundingNoGrantReason::FundingDisabled
        | FleetRootFundingNoGrantReason::InvalidRequest
        | FleetRootFundingNoGrantReason::PolicyMismatch
        | FleetRootFundingNoGrantReason::RegistryStale
        | FleetRootFundingNoGrantReason::RootAutomaticCapExhausted
        | FleetRootFundingNoGrantReason::RootIneligible
        | FleetRootFundingNoGrantReason::RootRejected => None,
    };
    match policy_reason {
        Some(reason) => policy::fleet_funding::permits_automatic_icp_fallback(reason),
        None => false,
    }
}

fn is_retryable_funding_error(err: &InternalError) -> bool {
    err.code() == codes::PLATFORM_FAILED
        || err.code() == codes::STATE_FAILED
        || err.public_error().code() == codes::STATE_CONFLICT.raw_code()
}

fn claim_resource_exhaustion_recovery(err: &InternalError) -> bool {
    err.is_public_resource_exhausted()
        && RESOURCE_EXHAUSTION_RECOVERY_CONSUMED.with(|consumed| !consumed.replace(true))
}

fn reset_resource_exhaustion_recovery() {
    RESOURCE_EXHAUSTION_RECOVERY_CONSUMED.with(|consumed| consumed.set(false));
}

const fn retryable_topup_after(delay: Duration) -> TimerRunResult {
    TimerRunResult::new(
        TimerCompletion::retryable_failure(0),
        TimerDirective::RetryAfter(delay),
    )
}

fn retry_delay(streak: u64) -> Duration {
    let exponent = u32::try_from(streak.min(5)).unwrap_or(5);
    let multiplier = 1u32 << exponent;
    RETRY_INITIAL
        .checked_mul(multiplier)
        .unwrap_or(RETRY_MAX)
        .min(RETRY_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_topup_retry_backoff_is_bounded_and_deterministic() {
        assert_eq!(retry_delay(0), Duration::from_mins(1));
        assert_eq!(retry_delay(1), Duration::from_mins(2));
        assert_eq!(retry_delay(4), Duration::from_mins(16));
        assert_eq!(retry_delay(5), Duration::from_mins(30));
        assert_eq!(retry_delay(u64::MAX), Duration::from_mins(30));
    }

    #[test]
    fn only_transport_and_in_flight_funding_failures_retry() {
        assert!(is_retryable_funding_error(&InternalError::state_failure()));
        assert!(is_retryable_funding_error(&InternalError::public(
            crate::diagnostics::codes::STATE_CONFLICT
        )));
        assert!(!is_retryable_funding_error(&InternalError::public(
            crate::diagnostics::codes::CAPACITY_LIMIT
        )));
        assert!(!is_retryable_funding_error(&InternalError::public(
            crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED
        )));
        assert!(!is_retryable_funding_error(&InternalError::invariant()));
    }

    #[test]
    fn automatic_refill_rate_gate_backs_off_but_balance_and_caps_stop() {
        assert_eq!(
            automatic_refill_failure_disposition(&InternalError::public(
                crate::diagnostics::codes::CAPACITY_INVALID_STATE
            )),
            AutomaticRefillFailureDisposition::BackOff
        );
        for code in [
            crate::diagnostics::codes::CAPACITY_UNAVAILABLE,
            crate::diagnostics::codes::CAPACITY_LIMIT,
            crate::diagnostics::codes::CAPACITY_INACTIVE,
        ] {
            assert_eq!(
                automatic_refill_failure_disposition(&InternalError::public(code)),
                AutomaticRefillFailureDisposition::StopOrRecover
            );
        }
    }

    #[test]
    fn resource_exhaustion_gets_one_recovery_attempt_between_successes() {
        let exhausted = InternalError::public(crate::diagnostics::codes::CAPACITY_LIMIT);
        let forbidden = InternalError::public(crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED);

        reset_resource_exhaustion_recovery();
        assert!(claim_resource_exhaustion_recovery(&exhausted));
        assert!(!claim_resource_exhaustion_recovery(&exhausted));
        assert!(!claim_resource_exhaustion_recovery(&forbidden));

        reset_resource_exhaustion_recovery();
        assert!(claim_resource_exhaustion_recovery(&exhausted));
        reset_resource_exhaustion_recovery();
    }

    #[test]
    fn nonroot_automatic_topup_remains_parent_funded() {
        let topup = TopupPolicy {
            threshold: Cycles::new(10),
            amount: Cycles::new(5),
        };

        let nonroot =
            select_automatic_topup_config(Some(topup), 300).expect("configured parent policy");
        assert_eq!(nonroot.threshold, 10);
        assert_eq!(nonroot.minimum_funding_spacing_secs, 300);
        assert!(nonroot.new_requests_enabled);
        assert!(matches!(
            nonroot.target,
            AutomaticTopupTarget::Parent { amount } if amount == Cycles::new(5)
        ));
        assert!(select_automatic_topup_config(None, 60).is_none());
    }

    #[test]
    fn automatic_topup_spacing_never_undercuts_the_observation_floor() {
        let nonroot = select_automatic_topup_config(Some(TopupPolicy::default()), 0)
            .expect("configured parent policy");

        assert_eq!(
            nonroot.minimum_funding_spacing_secs,
            policy::cycles::CYCLE_TOPUP_MIN_CHECK_SECS
        );
    }

    #[test]
    fn root_automatic_topup_uses_protected_schedule_and_runtime_switch() {
        let root = select_root_topup_config(
            RootFundingRuntimeConfig {
                request_threshold: 42,
                cooldown_secs: 0,
            },
            false,
        );

        assert_eq!(root.threshold, 42);
        assert_eq!(
            root.minimum_funding_spacing_secs,
            policy::cycles::CYCLE_TOPUP_MIN_CHECK_SECS
        );
        assert!(!root.new_requests_enabled);
        assert!(matches!(root.target, AutomaticTopupTarget::RootCoordinator));
    }

    #[test]
    fn durable_root_request_resumes_before_switch_or_balance_decisions() {
        let above_threshold = policy::cycles::CycleTopupTiming::CheckAfter { delay_secs: 900 };

        assert_eq!(
            select_topup_demand(true, false, above_threshold),
            AutomaticTopupDemand::Resume
        );
        assert_eq!(
            select_topup_demand(false, false, policy::cycles::CycleTopupTiming::Due),
            AutomaticTopupDemand::Stop
        );
        assert_eq!(
            select_topup_demand(false, true, policy::cycles::CycleTopupTiming::Due),
            AutomaticTopupDemand::Create
        );
        assert_eq!(
            select_topup_demand(false, true, above_threshold),
            AutomaticTopupDemand::Wait(above_threshold)
        );
    }

    #[test]
    fn root_no_grant_policy_separates_waits_from_invariant_rejections() {
        for reason in [
            FleetRootFundingNoGrantReason::CooldownActive,
            FleetRootFundingNoGrantReason::CoordinatorReserveUnavailable,
            FleetRootFundingNoGrantReason::FleetWindowExhausted,
            FleetRootFundingNoGrantReason::RootWindowExhausted,
        ] {
            assert_eq!(classify_root_no_grant(reason), RootNoGrantDisposition::Wait);
        }
        for reason in [
            FleetRootFundingNoGrantReason::InvalidRequest,
            FleetRootFundingNoGrantReason::FleetAutomaticCapExhausted,
            FleetRootFundingNoGrantReason::FundingDisabled,
            FleetRootFundingNoGrantReason::PolicyMismatch,
            FleetRootFundingNoGrantReason::RegistryStale,
            FleetRootFundingNoGrantReason::RootAutomaticCapExhausted,
            FleetRootFundingNoGrantReason::RootIneligible,
            FleetRootFundingNoGrantReason::RootRejected,
        ] {
            assert_eq!(classify_root_no_grant(reason), RootNoGrantDisposition::Stop);
        }
    }

    #[test]
    fn only_terminal_treasury_unavailability_permits_automatic_icp_fallback() {
        for reason in [
            FleetRootFundingNoGrantReason::CoordinatorReserveUnavailable,
            FleetRootFundingNoGrantReason::FleetWindowExhausted,
            FleetRootFundingNoGrantReason::RootWindowExhausted,
        ] {
            assert!(permits_automatic_refill(reason));
        }
        for reason in [
            FleetRootFundingNoGrantReason::CooldownActive,
            FleetRootFundingNoGrantReason::FleetAutomaticCapExhausted,
            FleetRootFundingNoGrantReason::FundingDisabled,
            FleetRootFundingNoGrantReason::InvalidRequest,
            FleetRootFundingNoGrantReason::PolicyMismatch,
            FleetRootFundingNoGrantReason::RegistryStale,
            FleetRootFundingNoGrantReason::RootAutomaticCapExhausted,
            FleetRootFundingNoGrantReason::RootIneligible,
            FleetRootFundingNoGrantReason::RootRejected,
        ] {
            assert!(!permits_automatic_refill(reason));
        }
    }

    #[test]
    fn automatic_topup_deadline_overflow_fails_closed() {
        assert!(CycleWorkflow::deadline_after_secs(u64::MAX, 1).is_err());
        assert!(CycleWorkflow::deadline_after_secs(0, u64::MAX).is_err());
    }
}
