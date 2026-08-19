//! Root-owned maintenance for prepaid empty Canisters on one physical Subnet.

mod refill;

use crate::ops::{
    canister_pool::CanisterPoolOps, component_registry::ComponentRegistryOps,
    storage::state::root_wasm_store::RootWasmStoreStateOps,
};
use canic_core::{
    api::timer::{TimerApi, TimerError as AuthorityTimerError},
    cdk::types::{Cycles, Principal},
    control_plane_support::{
        error::InternalError,
        ops::async_job_recovery::{
            AsyncJobAttempt, AsyncJobClaim, AsyncJobCompletion, AsyncJobOwner, AsyncJobRecoveryOps,
        },
        ops::ic::{
            IcOps,
            build_network::BuildNetworkOps,
            mgmt::{CanisterSettings, MgmtOps, UpdateSettingsArgs},
            nns::NnsRegistryOps,
        },
        workflow::runtime::fleet_activation::FleetActivationWorkflow,
    },
    dto::{
        fleet_activation::FleetActivationPhase,
        pool::{
            CanisterPoolResponse, CanisterPoolStatusRequest, PoolAdminCommand, PoolAdminResponse,
        },
    },
    ids::{BuildNetwork, FleetSubnetCanisterPoolConfig, SubnetId},
};
use ic_timers::{
    AfterCompletionRegistration, TimerCadence, TimerCompletion, TimerCompletionOutcome,
    TimerDirective, TimerIdentity, TimerReconcileState, TimerRunResult, WatchdogDecision,
    WatchdogRegistration, WatchdogRunResult, reconcile_after_completion, reconcile_watchdog,
};
use std::{
    cell::{Cell, RefCell},
    time::Duration,
};

const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(30);
const MAINTENANCE_LEASE_NS: u64 = 5 * 60 * 1_000_000_000;
const MAX_STATUS_PAGE_ENTRIES: u16 = 256;

thread_local! {
    static MAINTENANCE_TIMER: RefCell<Option<AfterCompletionRegistration>> = const { RefCell::new(None) };
    static RECOVERY_WATCHDOG: RefCell<Option<WatchdogRegistration>> = const { RefCell::new(None) };
    static MAINTENANCE_ENABLED: Cell<bool> = const { Cell::new(false) };
}

/// Reserve inactive maintenance in the shared inventory before application hooks run.
pub fn declare() {
    reconcile_native_timers(TimerReconcileState::Inactive).unwrap_or_else(|error| {
        ic_cdk::trap(format!(
            "canister-pool maintenance declaration rejected: {error}"
        ))
    });
    MAINTENANCE_ENABLED.set(true);
}

/// Start one non-overlapping root-owned maintenance loop.
pub fn start() -> Result<(), InternalError> {
    TimerApi::require_active().map_err(|_error| InternalError::invariant())?;
    reconcile_native_timers(TimerReconcileState::Scheduled)
        .map_err(|_error| InternalError::invariant())?;
    MAINTENANCE_ENABLED.set(true);
    TimerApi::defer_lifecycle_result_required(
        Duration::ZERO,
        "canic:canister_pool:maintain_initial",
        async { run_maintenance_timer().await },
    );
    Ok(())
}

/// Re-arm maintenance after snapshot resume only when the domain still owns it.
pub fn resume_after_authority_snapshot() -> Result<(), AuthorityTimerError> {
    if !MAINTENANCE_ENABLED.get() {
        return Ok(());
    }
    TimerApi::require_active()?;
    reconcile_native_timers(TimerReconcileState::Scheduled)
}

/// Cancel Root-owned claims before core authority owners are suspended.
pub fn suspend_for_authority_snapshot() -> Result<(), AuthorityTimerError> {
    cancel_native_timers()
}

/// Stop proactive maintenance once root draining has fenced new allocations.
pub fn stop() -> Result<(), InternalError> {
    if MAINTENANCE_ENABLED.replace(false) {
        cancel_native_timers().map_err(|_error| InternalError::invariant())?;
    }
    AsyncJobRecoveryOps::abandon(AsyncJobOwner::CanisterPoolMaintenance);
    Ok(())
}

/// Return the exact immutable policy and durable asset inventory.
pub fn status(request: CanisterPoolStatusRequest) -> Result<CanisterPoolResponse, InternalError> {
    if request.limit == 0 || request.limit > MAX_STATUS_PAGE_ENTRIES {
        return Err(InternalError::invalid_input());
    }
    Ok(CanisterPoolOps::response(
        pool_config()?,
        request.start_after,
        usize::from(request.limit),
    ))
}

/// Execute one controller-authorized maintenance command.
pub async fn admin(command: PoolAdminCommand) -> Result<PoolAdminResponse, InternalError> {
    match command {
        PoolAdminCommand::Maintain => maintain_once().await,
        PoolAdminCommand::RetryRefill => {
            if root_is_draining() {
                return Err(InternalError::conflict());
            }
            refill::retry_blocked()
        }
        PoolAdminCommand::Import { canister_id } => import(canister_id).await,
        PoolAdminCommand::RetryReset { canister_id } => {
            CanisterPoolOps::retry_reset(canister_id, IcOps::now_nanos())?;
            Ok(PoolAdminResponse::ResetQueued { canister_id })
        }
        PoolAdminCommand::Handoff {
            canister_id,
            recipient,
        } => handoff(canister_id, recipient).await,
    }
}

/// Reconcile one bounded reset or automatic refill operation.
pub async fn maintain_once() -> Result<PoolAdminResponse, InternalError> {
    let attempt = match claim_maintenance()? {
        AsyncJobClaim::Acquired(attempt) => attempt,
        AsyncJobClaim::Busy { .. } => {
            return Ok(PoolAdminResponse::MaintenancePaused {
                reason: "another Canister pool maintenance pass is still in flight".to_string(),
            });
        }
    };
    let result = maintain_once_inner().await;
    let completion = maintenance_result_completion(&result);
    if !AsyncJobRecoveryOps::finish(attempt, completion)? {
        return Err(InternalError::invariant());
    }
    result
}

async fn maintain_once_inner() -> Result<PoolAdminResponse, InternalError> {
    let status = FleetActivationWorkflow::status()?;
    if !matches!(
        status.phase,
        FleetActivationPhase::Prepared | FleetActivationPhase::Active
    ) {
        return Ok(PoolAdminResponse::MaintenancePaused {
            reason: "Canister pool maintenance requires a Prepared or Active Fleet Subnet Root"
                .to_string(),
        });
    }
    let config = pool_config()?;

    if CanisterPoolOps::pending_handoff().is_some() {
        return Ok(PoolAdminResponse::MaintenancePaused {
            reason: "Canister pool asset handoff is pending".to_string(),
        });
    }

    if CanisterPoolOps::pending_creation().is_some() {
        return if root_is_draining() {
            refill::reconcile_draining().await
        } else {
            refill::reconcile().await
        };
    }

    if let Some(canister_id) = CanisterPoolOps::pending_reset_canisters()
        .into_iter()
        .next()
    {
        return Ok(reset_admin_response(
            canister_id,
            reset_asset(canister_id, &config).await?,
        ));
    }
    if root_is_draining() {
        return Ok(PoolAdminResponse::MaintenancePaused {
            reason: "Fleet Subnet Root draining has fenced pool replenishment".to_string(),
        });
    }
    if CanisterPoolOps::ready_count() >= config.minimum_size {
        return Ok(PoolAdminResponse::Maintained);
    }
    refill::start(&config).await
}

async fn run_maintenance_timer() -> TimerRunResult {
    let attempt = match claim_maintenance() {
        Ok(AsyncJobClaim::Acquired(attempt)) => attempt,
        Ok(AsyncJobClaim::Busy { retry_at_ns }) => {
            return TimerRunResult::new(
                TimerCompletion::retryable_failure(0),
                TimerDirective::ScheduleAt(retry_at_ns),
            );
        }
        Err(_) => {
            return TimerRunResult::new(
                TimerCompletion::invariant_failure(0),
                TimerDirective::Stop,
            );
        }
    };
    finish_maintenance_timer(attempt, maintain_once_inner().await)
}

/// Dispatch one watchdog-owned takeover without awaiting fallible work.
pub fn dispatch_async_job_recovery() -> bool {
    let attempt = match claim_maintenance() {
        Ok(AsyncJobClaim::Acquired(attempt)) => attempt,
        Ok(AsyncJobClaim::Busy { .. }) | Err(_) => return false,
    };
    spawn_maintenance(attempt);
    true
}

fn run_recovery_watchdog() -> WatchdogRunResult {
    let now_ns = IcOps::now_nanos();
    let mut recovered = TimerApi::recover_expired_async_jobs(now_ns);
    if AsyncJobRecoveryOps::expired_deadline(AsyncJobOwner::CanisterPoolMaintenance, now_ns)
        .is_some()
        && dispatch_async_job_recovery()
    {
        recovered = recovered.saturating_add(1);
    }
    let completion = if recovered == 0 {
        TimerCompletion::no_work()
    } else {
        TimerCompletion::success(recovered)
    };
    WatchdogRunResult::new(completion, WatchdogDecision::Continue)
}

fn reconcile_native_timers(desired: TimerReconcileState) -> Result<(), AuthorityTimerError> {
    reconcile_maintenance_timer(desired)?;
    if let Err(primary) = reconcile_recovery_watchdog(desired) {
        return match cancel_maintenance_timer() {
            Ok(()) => Err(primary),
            Err(cleanup) => Err(AuthorityTimerError::RegistrationRollback {
                primary: Box::new(primary),
                cleanup: Box::new(cleanup),
            }),
        };
    }
    Ok(())
}

fn reconcile_maintenance_timer(desired: TimerReconcileState) -> Result<(), AuthorityTimerError> {
    let identity = TimerIdentity::try_new("canic", "canister_pool", "maintain")?;
    let cadence = TimerCadence::new(MAINTENANCE_INTERVAL)?;
    MAINTENANCE_TIMER
        .try_with(|registration| {
            let mut registration = registration
                .try_borrow_mut()
                .map_err(|_| AuthorityTimerError::CustodyBusy)?;
            reconcile_after_completion(
                &mut registration,
                &identity,
                cadence,
                desired,
                |_context| async {
                    let result = run_maintenance_timer().await;
                    TimerRunResult::new(result.completion(), TimerDirective::RecurAfterCompletion)
                },
            )
            .map_err(AuthorityTimerError::from)
        })
        .map_err(|_| AuthorityTimerError::CustodyBusy)?
}

fn reconcile_recovery_watchdog(desired: TimerReconcileState) -> Result<(), AuthorityTimerError> {
    let identity = TimerApi::recovery_watchdog_identity()?;
    let cadence = TimerCadence::new(MAINTENANCE_INTERVAL)?;
    RECOVERY_WATCHDOG
        .try_with(|registration| {
            let mut registration = registration
                .try_borrow_mut()
                .map_err(|_| AuthorityTimerError::CustodyBusy)?;
            reconcile_watchdog(&mut registration, &identity, cadence, desired, |_context| {
                run_recovery_watchdog()
            })
            .map_err(AuthorityTimerError::from)
        })
        .map_err(|_| AuthorityTimerError::CustodyBusy)?
}

fn cancel_native_timers() -> Result<(), AuthorityTimerError> {
    cancel_maintenance_timer()?;
    RECOVERY_WATCHDOG
        .try_with(|registration| {
            let registration = registration
                .try_borrow()
                .map_err(|_| AuthorityTimerError::CustodyBusy)?;
            if let Some(registration) = registration.as_ref() {
                registration.cancel()?;
            }
            Ok(())
        })
        .map_err(|_| AuthorityTimerError::CustodyBusy)?
}

fn cancel_maintenance_timer() -> Result<(), AuthorityTimerError> {
    MAINTENANCE_TIMER
        .try_with(|registration| {
            let registration = registration
                .try_borrow()
                .map_err(|_| AuthorityTimerError::CustodyBusy)?;
            if let Some(registration) = registration.as_ref() {
                registration.cancel()?;
            }
            Ok(())
        })
        .map_err(|_| AuthorityTimerError::CustodyBusy)?
}

fn spawn_maintenance(attempt: AsyncJobAttempt) {
    ic_cdk::futures::spawn(async move {
        let _result = finish_maintenance_timer(attempt, maintain_once_inner().await);
    });
}

fn claim_maintenance() -> Result<AsyncJobClaim, InternalError> {
    let now_ns = IcOps::now_nanos();
    let lease_expires_at_ns = now_ns
        .checked_add(MAINTENANCE_LEASE_NS)
        .ok_or_else(InternalError::invariant)?;
    AsyncJobRecoveryOps::claim(
        AsyncJobOwner::CanisterPoolMaintenance,
        now_ns,
        lease_expires_at_ns,
    )
}

fn finish_maintenance_timer(
    attempt: AsyncJobAttempt,
    result: Result<PoolAdminResponse, InternalError>,
) -> TimerRunResult {
    let timer_result = maintenance_timer_result(result);
    let completion = timer_result_completion(timer_result.completion().outcome());
    let Ok(exact) = AsyncJobRecoveryOps::finish(attempt, completion) else {
        return TimerRunResult::new(TimerCompletion::invariant_failure(0), TimerDirective::Stop);
    };
    if exact && completion == AsyncJobCompletion::InvariantFailure {
        MAINTENANCE_ENABLED.set(false);
        let _ = cancel_native_timers();
    }
    if exact {
        timer_result
    } else {
        TimerRunResult::new(TimerCompletion::no_work(), TimerDirective::Stop)
    }
}

const fn maintenance_result_completion(
    result: &Result<PoolAdminResponse, InternalError>,
) -> AsyncJobCompletion {
    match result {
        Ok(_) => AsyncJobCompletion::Success,
        Err(error) if is_retryable_maintenance_error(error) => AsyncJobCompletion::RetryableFailure,
        Err(_) => AsyncJobCompletion::InvariantFailure,
    }
}

const fn timer_result_completion(outcome: TimerCompletionOutcome) -> AsyncJobCompletion {
    match outcome {
        TimerCompletionOutcome::Success | TimerCompletionOutcome::NoWork => {
            AsyncJobCompletion::Success
        }
        TimerCompletionOutcome::RetryableFailure => AsyncJobCompletion::RetryableFailure,
        TimerCompletionOutcome::InvariantFailure => AsyncJobCompletion::InvariantFailure,
    }
}

fn maintenance_timer_result(result: Result<PoolAdminResponse, InternalError>) -> TimerRunResult {
    match result {
        Ok(PoolAdminResponse::MaintenancePaused { .. }) => {
            TimerRunResult::new(TimerCompletion::no_work(), TimerDirective::Stop)
        }
        Ok(_) => TimerRunResult::new(TimerCompletion::success(1), TimerDirective::Stop),
        Err(error) if is_retryable_maintenance_error(&error) => {
            TimerRunResult::new(TimerCompletion::retryable_failure(0), TimerDirective::Stop)
        }
        Err(_) => TimerRunResult::new(TimerCompletion::invariant_failure(0), TimerDirective::Stop),
    }
}

const fn is_retryable_maintenance_error(error: &InternalError) -> bool {
    let code = error.code().raw_code().raw();
    code == canic_core::diagnostics::codes::PLATFORM_FAILED
        .raw_code()
        .raw()
        || code
            == canic_core::diagnostics::codes::STATE_FAILED
                .raw_code()
                .raw()
}

async fn import(canister_id: Principal) -> Result<PoolAdminResponse, InternalError> {
    if root_is_draining() {
        return Err(InternalError::conflict());
    }
    require_import_candidate(canister_id)?;
    require_ic_import_on_root_subnet(canister_id).await?;
    let config = pool_config()?;
    CanisterPoolOps::initialize_imports(&config, &[canister_id], IcOps::now_nanos())?;
    match reset_asset(canister_id, &config).await? {
        ResetAssetOutcome::Ready => Ok(PoolAdminResponse::Imported { canister_id }),
        ResetAssetOutcome::Underfunded { reason } => Ok(PoolAdminResponse::ResetFailed {
            canister_id,
            reason,
        }),
    }
}

async fn handoff(
    canister_id: Principal,
    recipient: Principal,
) -> Result<PoolAdminResponse, InternalError> {
    if !root_is_draining() {
        return Err(InternalError::conflict());
    }
    let root = IcOps::canister_self();
    if recipient == Principal::anonymous()
        || recipient == Principal::management_canister()
        || recipient == root
        || recipient == canister_id
    {
        return Err(InternalError::invalid_input());
    }
    if let Some(existing) = CanisterPoolOps::completed_handoff_recipient(canister_id) {
        if existing == recipient {
            return Ok(PoolAdminResponse::HandedOff {
                canister_id,
                recipient,
            });
        }
        return Err(InternalError::conflict());
    }
    CanisterPoolOps::begin_handoff(canister_id, recipient, IcOps::now_nanos())?;
    MgmtOps::update_settings(&UpdateSettingsArgs {
        canister_id,
        settings: CanisterSettings {
            controllers: Some(vec![root, recipient]),
            ..CanisterSettings::default()
        },
        sender_canister_version: None,
    })
    .await?;
    CanisterPoolOps::complete_handoff(canister_id, recipient, IcOps::now_nanos())?;
    Ok(PoolAdminResponse::HandedOff {
        canister_id,
        recipient,
    })
}

fn require_import_candidate(canister_id: Principal) -> Result<(), InternalError> {
    let root = FleetActivationWorkflow::root_authority()?.binding;
    if canister_id == root.fleet_subnet_root
        || canister_id == root.authority.binding.coordinator
        || RootWasmStoreStateOps::wasm_stores()
            .iter()
            .any(|store| store.pid == canister_id)
    {
        return Err(InternalError::conflict());
    }
    if ComponentRegistryOps::component_for_principal(canister_id).is_some() {
        return Err(InternalError::conflict());
    }
    Ok(())
}

/// Return a stopped Component Canister to durable local prepaid inventory.
pub async fn recycle(canister_id: Principal) -> Result<(), InternalError> {
    let config = pool_config()?;
    CanisterPoolOps::register_recycled_pending(canister_id, IcOps::now_nanos())?;
    if CanisterPoolOps::recycling_reset_is_terminal(canister_id)? {
        return Ok(());
    }
    let _ = reset_asset(canister_id, &config).await?;
    Ok(())
}

fn root_is_draining() -> bool {
    ComponentRegistryOps::current().is_some_and(|registry| registry.root_draining.is_some())
}

enum ResetAssetOutcome {
    Ready,
    Underfunded { reason: String },
}

async fn reset_asset(
    canister_id: Principal,
    config: &FleetSubnetCanisterPoolConfig,
) -> Result<ResetAssetOutcome, InternalError> {
    if CanisterPoolOps::asset_is_ready(canister_id)? {
        return Ok(ResetAssetOutcome::Ready);
    }
    let root = IcOps::canister_self();
    let result: Result<Cycles, InternalError> = async {
        MgmtOps::update_settings(&UpdateSettingsArgs {
            canister_id,
            settings: CanisterSettings {
                controllers: Some(vec![root]),
                ..CanisterSettings::default()
            },
            sender_canister_version: None,
        })
        .await?;
        MgmtOps::uninstall_code(canister_id).await?;
        let cycles = MgmtOps::get_cycles(canister_id).await?;
        Ok(cycles)
    }
    .await;

    match result {
        Ok(cycles) if cycles >= config.canister_cycles => {
            CanisterPoolOps::mark_ready(canister_id, cycles, IcOps::now_nanos())?;
            Ok(ResetAssetOutcome::Ready)
        }
        Ok(cycles) => {
            let reason = format!(
                "Canister pool asset {canister_id} has {cycles}, below configured {}",
                config.canister_cycles
            );
            CanisterPoolOps::mark_failed(
                canister_id,
                Some(cycles),
                reason.clone(),
                IcOps::now_nanos(),
            )?;
            Ok(ResetAssetOutcome::Underfunded { reason })
        }
        Err(error) => {
            CanisterPoolOps::mark_failed(canister_id, None, error.to_string(), IcOps::now_nanos())?;
            Err(error)
        }
    }
}

fn reset_admin_response(canister_id: Principal, outcome: ResetAssetOutcome) -> PoolAdminResponse {
    match outcome {
        ResetAssetOutcome::Ready => PoolAdminResponse::ResetReady { canister_id },
        ResetAssetOutcome::Underfunded { reason } => PoolAdminResponse::ResetFailed {
            canister_id,
            reason,
        },
    }
}

async fn require_ic_import_on_root_subnet(canister_id: Principal) -> Result<(), InternalError> {
    if BuildNetworkOps::build_network() != Some(BuildNetwork::Ic) {
        return Ok(());
    }
    let expected = FleetActivationWorkflow::root_authority()?
        .binding
        .placement_subnet;
    let actual = NnsRegistryOps::get_subnet_for_canister(canister_id).await?;
    validate_import_subnet(canister_id, expected, actual)
}

fn validate_import_subnet(
    _canister_id: Principal,
    expected: SubnetId,
    actual: Option<Principal>,
) -> Result<(), InternalError> {
    let actual = actual.ok_or_else(InternalError::unavailable)?;
    if actual != expected.into_principal() {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn pool_config() -> Result<FleetSubnetCanisterPoolConfig, InternalError> {
    Ok(FleetActivationWorkflow::root_authority()?
        .binding
        .limits
        .canister_pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_subnet_requires_exact_nns_routing_evidence() {
        let canister_id = Principal::from_slice(&[3; 29]);
        let expected = SubnetId::from_principal(Principal::from_slice(&[4; 29]));
        assert!(
            validate_import_subnet(canister_id, expected, Some(expected.into_principal())).is_ok()
        );
        assert!(validate_import_subnet(canister_id, expected, None).is_err());
        assert!(
            validate_import_subnet(canister_id, expected, Some(Principal::from_slice(&[5; 29])))
                .is_err()
        );
    }

    #[test]
    fn maintenance_timer_completion_preserves_domain_outcomes() {
        let paused = maintenance_timer_result(Ok(PoolAdminResponse::MaintenancePaused {
            reason: "not active".to_string(),
        }));
        assert_eq!(
            paused.completion().outcome(),
            TimerCompletionOutcome::NoWork
        );

        let retryable = maintenance_timer_result(Err(InternalError::platform_failure()));
        assert_eq!(
            retryable.completion().outcome(),
            TimerCompletionOutcome::RetryableFailure
        );

        let invariant = maintenance_timer_result(Err(InternalError::invariant()));
        assert_eq!(
            invariant.completion().outcome(),
            TimerCompletionOutcome::InvariantFailure
        );
    }
}
