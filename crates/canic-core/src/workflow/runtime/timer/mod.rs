//! Module: workflow::runtime::timer
//!
//! Responsibility: own runtime timer identity, arbitration, recurrence, and live status.
//! Does not own: domain work predicates, stable queues, retry policy, or lifecycle hooks.
//! Boundary: all canister scheduling reaches the IC only through TimerOps.

mod control;

use self::control::{TimerControl, TimerControlAction, TimerRegistration};
use crate::{
    domain::runtime::{
        TimerExecutionOutcome, TimerMode, TimerProcessCondition, TimerRegistrationStatus,
        TimerSchedulingMode,
    },
    ops::{ic::IcOps, runtime::timer::TimerOps},
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    rc::Rc,
    time::Duration,
};

use crate::ops::runtime::timer::TimerId;

type TimerFuture = Pin<Box<dyn Future<Output = TimerRunResult>>>;
type TimerTaskFactory = Rc<RefCell<dyn FnMut() -> TimerFuture>>;

thread_local! {
    static TIMERS: RefCell<BTreeMap<TimerIdentity, TimerEntry>> = const { RefCell::new(BTreeMap::new()) };
    static NEXT_APPLICATION_TIMER_ID: Cell<u64> = const { Cell::new(0) };
}

/// Closed identities for Canic-owned background processes.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum TimerKey {
    AuthRenewal,
    CycleTopup,
    IntentCleanup,
    LogRetention,
    PlacementReceiptAcknowledgement,
    PoolReset,
}

impl TimerKey {
    const fn label(self) -> &'static str {
        match self {
            Self::AuthRenewal => "auth_renewal:run",
            Self::CycleTopup => "cycles:topup",
            Self::IntentCleanup => "intent_cleanup:run",
            Self::LogRetention => "log_retention:run",
            Self::PlacementReceiptAcknowledgement => "placement:receipt_ack",
            Self::PoolReset => "pool:pending",
        }
    }
}

/// Opaque identity returned to the public application timer facade.
#[derive(Debug, Eq, PartialEq)]
pub struct ApplicationTimerId(u64);

/// Scheduling decision returned after one bounded built-in invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimerDirective {
    Stop,
    ContinueImmediately,
    RetryAfter(Duration),
    ScheduleAt(u64),
    RecurAfter(Duration),
}

impl TimerDirective {
    fn deadline_and_mode(
        self,
        now_ns: u64,
    ) -> Result<Option<(u64, TimerSchedulingMode)>, &'static str> {
        match self {
            Self::Stop => Ok(None),
            Self::ContinueImmediately => Ok(Some((now_ns, TimerSchedulingMode::Continuation))),
            Self::RetryAfter(delay) => deadline_after(now_ns, delay)
                .map(|deadline| Some((deadline, TimerSchedulingMode::Retry))),
            Self::ScheduleAt(deadline) => Ok(Some((deadline, TimerSchedulingMode::Deadline))),
            Self::RecurAfter(delay) => deadline_after(now_ns, delay)
                .map(|deadline| Some((deadline, TimerSchedulingMode::AfterCompletion))),
        }
    }
}

/// Typed result of one bounded built-in invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerRunResult {
    pub outcome: TimerExecutionOutcome,
    pub work_count: u64,
    pub directive: TimerDirective,
}

impl TimerRunResult {
    #[must_use]
    pub const fn success(work_count: u64, directive: TimerDirective) -> Self {
        Self {
            outcome: TimerExecutionOutcome::Success,
            work_count,
            directive,
        }
    }

    #[must_use]
    pub const fn no_work(directive: TimerDirective) -> Self {
        Self {
            outcome: TimerExecutionOutcome::NoWork,
            work_count: 0,
            directive,
        }
    }

    #[must_use]
    pub const fn invariant_failure() -> Self {
        Self {
            outcome: TimerExecutionOutcome::InvariantFailure,
            work_count: 0,
            directive: TimerDirective::Stop,
        }
    }
}

/// Heap-only timer status, explicitly scoped to the current runtime start.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimerRuntimeSnapshot {
    pub label: String,
    pub scheduling_mode: TimerSchedulingMode,
    pub registration: TimerRegistrationStatus,
    pub condition: TimerProcessCondition,
    pub enabled: bool,
    pub generation: u64,
    pub next_due_at_ns: Option<u64>,
    pub last_outcome: Option<TimerExecutionOutcome>,
    pub last_work_count: u64,
    pub last_success_at_ns: Option<u64>,
    pub last_failure_at_ns: Option<u64>,
    pub consecutive_expected_failures: u64,
    pub schedules_since_runtime_start: u64,
    pub executions_since_runtime_start: u64,
    pub successes_since_runtime_start: u64,
    pub expected_failures_since_runtime_start: u64,
    pub invariant_failures_since_runtime_start: u64,
    pub stale_callbacks_since_runtime_start: u64,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum TimerIdentity {
    BuiltIn(TimerKey),
    Application(u64),
}

struct TimerEntry {
    label: String,
    timer_mode: TimerMode,
    scheduling_mode: TimerSchedulingMode,
    enabled: bool,
    condition: TimerProcessCondition,
    retain_when_stopped: bool,
    control: TimerControl,
    handle: Option<TimerId>,
    task: TimerTaskFactory,
    last_outcome: Option<TimerExecutionOutcome>,
    last_work_count: u64,
    last_success_at_ns: Option<u64>,
    last_failure_at_ns: Option<u64>,
    consecutive_expected_failures: u64,
    schedules: u64,
    executions: u64,
    successes: u64,
    expected_failures: u64,
    invariant_failures: u64,
    stale_callbacks: u64,
}

impl TimerEntry {
    fn new(
        label: String,
        timer_mode: TimerMode,
        scheduling_mode: TimerSchedulingMode,
        retain_when_stopped: bool,
        task: TimerTaskFactory,
    ) -> Self {
        Self {
            label,
            timer_mode,
            scheduling_mode,
            enabled: true,
            condition: TimerProcessCondition::Active,
            retain_when_stopped,
            control: TimerControl::default(),
            handle: None,
            task,
            last_outcome: None,
            last_work_count: 0,
            last_success_at_ns: None,
            last_failure_at_ns: None,
            consecutive_expected_failures: 0,
            schedules: 0,
            executions: 0,
            successes: 0,
            expected_failures: 0,
            invariant_failures: 0,
            stale_callbacks: 0,
        }
    }

    fn snapshot(&self) -> TimerRuntimeSnapshot {
        let (registration, next_due_at_ns) = match self.control.registration() {
            TimerRegistration::Unregistered => (TimerRegistrationStatus::Unregistered, None),
            TimerRegistration::Scheduled { deadline_ns, .. } => {
                (TimerRegistrationStatus::Scheduled, Some(deadline_ns))
            }
            TimerRegistration::Running { .. } => (TimerRegistrationStatus::Running, None),
        };

        TimerRuntimeSnapshot {
            label: self.label.clone(),
            scheduling_mode: self.scheduling_mode,
            registration,
            condition: self.condition,
            enabled: self.enabled,
            generation: self.control.generation(),
            next_due_at_ns,
            last_outcome: self.last_outcome,
            last_work_count: self.last_work_count,
            last_success_at_ns: self.last_success_at_ns,
            last_failure_at_ns: self.last_failure_at_ns,
            consecutive_expected_failures: self.consecutive_expected_failures,
            schedules_since_runtime_start: self.schedules,
            executions_since_runtime_start: self.executions,
            successes_since_runtime_start: self.successes,
            expected_failures_since_runtime_start: self.expected_failures,
            invariant_failures_since_runtime_start: self.invariant_failures,
            stale_callbacks_since_runtime_start: self.stale_callbacks,
        }
    }
}

/// Canonical scheduling workflow for built-in and application timers.
pub struct TimerWorkflow;

impl TimerWorkflow {
    /// Schedule a cancellable application one-shot.
    pub fn set_application_once(
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = ()> + 'static,
    ) -> ApplicationTimerId {
        let id = next_application_timer_id();
        let mut task = Some(task);
        let factory = timer_factory(move || {
            let task = task
                .take()
                .unwrap_or_else(|| panic!("application one-shot executed more than once"));
            async move {
                task.await;
                TimerRunResult::success(1, TimerDirective::Stop)
            }
        });
        let identity = TimerIdentity::Application(id);
        insert_entry(
            identity,
            TimerEntry::new(
                label.into(),
                TimerMode::Once,
                TimerSchedulingMode::Once,
                false,
                factory,
            ),
        );
        request_after(identity, delay);
        ApplicationTimerId(id)
    }

    /// Schedule a cancellable application interval after each completed invocation.
    pub fn set_application_interval<F, Fut>(
        interval: Duration,
        label: impl Into<String>,
        task: F,
    ) -> ApplicationTimerId
    where
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        let id = next_application_timer_id();
        let mut task = task;
        let factory = timer_factory(move || {
            let future = task();
            async move {
                future.await;
                TimerRunResult::success(1, TimerDirective::RecurAfter(interval))
            }
        });
        let identity = TimerIdentity::Application(id);
        insert_entry(
            identity,
            TimerEntry::new(
                label.into(),
                TimerMode::Interval,
                TimerSchedulingMode::AfterCompletion,
                false,
                factory,
            ),
        );
        request_after(identity, interval);
        ApplicationTimerId(id)
    }

    /// Consume an application timer identity and suppress any future invocation.
    #[must_use]
    pub fn cancel_application(id: ApplicationTimerId) -> bool {
        cancel(TimerIdentity::Application(id.0))
    }

    /// Configure one built-in bounded process if needed and request its next run.
    pub fn schedule<F, Fut>(key: TimerKey, delay: Duration, task: F)
    where
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = TimerRunResult> + 'static,
    {
        ensure_builtin(
            key,
            delay,
            TimerMode::Once,
            TimerSchedulingMode::Once,
            timer_factory(task),
        );
    }

    /// Configure one built-in bounded process if needed and request an absolute deadline.
    pub fn schedule_at<F, Fut>(key: TimerKey, deadline_ns: u64, task: F)
    where
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = TimerRunResult> + 'static,
    {
        let identity = TimerIdentity::BuiltIn(key);
        ensure_bounded_entry(identity, key, task);
        request_at(identity, deadline_ns);
    }

    /// Configure one built-in bounded process and reconcile its sole live deadline.
    pub fn reconcile_at<F, Fut>(key: TimerKey, deadline_ns: Option<u64>, task: F)
    where
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = TimerRunResult> + 'static,
    {
        let identity = TimerIdentity::BuiltIn(key);
        ensure_bounded_entry(identity, key, task);
        match deadline_ns {
            Some(deadline_ns) => request_reconcile_at(identity, deadline_ns),
            None => request_idle(identity),
        }
    }

    /// Snapshot live registrations and current-runtime counters.
    #[must_use]
    pub fn statuses() -> Vec<TimerRuntimeSnapshot> {
        TIMERS.with_borrow(|timers| timers.values().map(TimerEntry::snapshot).collect())
    }

    /// Return the completed retryable-failure streak for one built-in owner.
    #[must_use]
    pub(crate) fn consecutive_expected_failures(key: TimerKey) -> u64 {
        TIMERS.with_borrow(|timers| {
            timers
                .get(&TimerIdentity::BuiltIn(key))
                .map_or(0, |entry| entry.consecutive_expected_failures)
        })
    }
}

fn ensure_bounded_entry<F, Fut>(identity: TimerIdentity, key: TimerKey, task: F)
where
    F: FnMut() -> Fut + 'static,
    Fut: Future<Output = TimerRunResult> + 'static,
{
    insert_entry_if_absent(
        identity,
        TimerEntry::new(
            key.label().to_string(),
            TimerMode::Once,
            TimerSchedulingMode::Deadline,
            true,
            timer_factory(task),
        ),
    );
}

fn ensure_builtin(
    key: TimerKey,
    initial_delay: Duration,
    timer_mode: TimerMode,
    scheduling_mode: TimerSchedulingMode,
    factory: TimerTaskFactory,
) -> bool {
    let identity = TimerIdentity::BuiltIn(key);
    let inserted = insert_entry_if_absent(
        identity,
        TimerEntry::new(
            key.label().to_string(),
            timer_mode,
            scheduling_mode,
            true,
            factory,
        ),
    );
    request_after(identity, initial_delay);
    inserted
}

fn timer_factory<F, Fut>(mut task: F) -> TimerTaskFactory
where
    F: FnMut() -> Fut + 'static,
    Fut: Future<Output = TimerRunResult> + 'static,
{
    Rc::new(RefCell::new(move || {
        let future: TimerFuture = Box::pin(task());
        future
    }))
}

fn insert_entry(identity: TimerIdentity, entry: TimerEntry) {
    let previous = TIMERS.with_borrow_mut(|timers| timers.insert(identity, entry));
    assert!(
        previous.is_none(),
        "timer identity allocated more than once"
    );
}

fn insert_entry_if_absent(identity: TimerIdentity, entry: TimerEntry) -> bool {
    TIMERS.with_borrow_mut(|timers| {
        if let std::collections::btree_map::Entry::Vacant(vacant) = timers.entry(identity) {
            vacant.insert(entry);
            return true;
        }
        false
    })
}

fn request_after(identity: TimerIdentity, delay: Duration) {
    let now_ns = IcOps::now_nanos();
    let deadline_ns = deadline_after(now_ns, delay)
        .unwrap_or_else(|err| panic!("timer scheduling failed closed: {err}"));
    request_at(identity, deadline_ns);
}

fn request_at(identity: TimerIdentity, deadline_ns: u64) {
    let action = TIMERS.with_borrow_mut(|timers| {
        let entry = timers
            .get_mut(&identity)
            .unwrap_or_else(|| panic!("timer identity is not configured"));
        entry.enabled = true;
        entry.condition = TimerProcessCondition::Active;
        let action = entry
            .control
            .schedule(deadline_ns)
            .unwrap_or_else(|err| panic!("timer scheduling failed closed: {err}"));
        if matches!(
            action,
            TimerControlAction::Arm { .. } | TimerControlAction::Replace { .. }
        ) {
            entry.scheduling_mode = TimerSchedulingMode::Deadline;
        }
        action
    });
    apply_action(identity, action);
}

fn request_reconcile_at(identity: TimerIdentity, deadline_ns: u64) {
    let action = TIMERS.with_borrow_mut(|timers| {
        let entry = timers
            .get_mut(&identity)
            .unwrap_or_else(|| panic!("timer identity is not configured"));
        entry.enabled = true;
        entry.condition = TimerProcessCondition::Active;
        entry.scheduling_mode = TimerSchedulingMode::Deadline;
        entry
            .control
            .reconcile(deadline_ns)
            .unwrap_or_else(|err| panic!("timer deadline reconciliation failed closed: {err}"))
    });
    apply_action(identity, action);
}

fn request_idle(identity: TimerIdentity) {
    let action = TIMERS.with_borrow_mut(|timers| {
        let entry = timers
            .get_mut(&identity)
            .unwrap_or_else(|| panic!("timer identity is not configured"));
        entry.enabled = true;
        entry.condition = TimerProcessCondition::Idle;
        entry
            .control
            .cancel()
            .unwrap_or_else(|err| panic!("timer idle reconciliation failed closed: {err}"))
    });
    apply_action(identity, action);
}

fn cancel(identity: TimerIdentity) -> bool {
    let Some(action) = TIMERS.with_borrow_mut(|timers| {
        let entry = timers.get_mut(&identity)?;
        entry.enabled = false;
        entry.condition = TimerProcessCondition::Disabled;
        Some(
            entry
                .control
                .cancel()
                .unwrap_or_else(|err| panic!("timer cancellation failed closed: {err}")),
        )
    }) else {
        return false;
    };
    apply_action(identity, action);
    let remove = TIMERS.with_borrow(|timers| {
        timers.get(&identity).is_some_and(|entry| {
            !entry.retain_when_stopped
                && matches!(
                    entry.control.registration(),
                    TimerRegistration::Unregistered
                )
        })
    });
    if remove {
        TIMERS.with_borrow_mut(|timers| {
            timers.remove(&identity);
        });
    }
    true
}

fn apply_action(identity: TimerIdentity, action: TimerControlAction) {
    match action {
        TimerControlAction::None | TimerControlAction::Disarm { .. } => {}
        TimerControlAction::Clear => clear_current_handle(identity),
        TimerControlAction::Replace {
            generation,
            deadline_ns,
        } => {
            clear_current_handle(identity);
            arm(identity, generation, deadline_ns);
        }
        TimerControlAction::Arm {
            generation,
            deadline_ns,
        } => arm(identity, generation, deadline_ns),
    }
}

fn clear_current_handle(identity: TimerIdentity) {
    if let Some(handle) = TIMERS.with_borrow_mut(|timers| {
        timers
            .get_mut(&identity)
            .and_then(|entry| entry.handle.take())
    }) {
        TimerOps::clear(handle);
    }
}

fn arm(identity: TimerIdentity, generation: u64, deadline_ns: u64) {
    let (label, timer_mode) = TIMERS.with_borrow(|timers| {
        let entry = timers
            .get(&identity)
            .unwrap_or_else(|| panic!("timer identity disappeared before arm"));
        (entry.label.clone(), entry.timer_mode)
    });
    let now_ns = IcOps::now_nanos();
    let delay = Duration::from_nanos(deadline_ns.saturating_sub(now_ns));
    let handle = TimerOps::set(delay, timer_mode, label, async move {
        fire(identity, generation).await;
    });

    let keep = TIMERS.with_borrow_mut(|timers| {
        let Some(entry) = timers.get_mut(&identity) else {
            return false;
        };
        if entry.control.registration()
            != (TimerRegistration::Scheduled {
                generation,
                deadline_ns,
            })
        {
            return false;
        }
        entry.handle = Some(handle);
        entry.schedules = entry.schedules.saturating_add(1);
        true
    });
    if !keep {
        TimerOps::clear(handle);
    }
}

#[expect(
    clippy::future_not_send,
    reason = "IC canister futures and thread-local timer factories are intentionally single-threaded"
)]
async fn fire(identity: TimerIdentity, generation: u64) {
    let task = TIMERS.with_borrow_mut(|timers| {
        let entry = timers.get_mut(&identity)?;
        if !entry.control.begin(generation) {
            entry.stale_callbacks = entry.stale_callbacks.saturating_add(1);
            return None;
        }
        entry.handle = None;
        entry.executions = entry.executions.saturating_add(1);
        Some(Rc::clone(&entry.task))
    });
    let Some(task) = task else {
        return;
    };

    let future = { (task.borrow_mut())() };
    drop(task);
    let mut result = future.await;
    let now_ns = IcOps::now_nanos();
    let deadline_and_mode = result.directive.deadline_and_mode(now_ns);
    if deadline_and_mode.is_err() {
        result = TimerRunResult::invariant_failure();
    }
    let (directive_deadline_ns, scheduling_mode) = deadline_and_mode.ok().flatten().unzip();

    let (action, remove) = TIMERS.with_borrow_mut(|timers| {
        let entry = timers
            .get_mut(&identity)
            .unwrap_or_else(|| panic!("running timer identity disappeared"));
        entry.last_outcome = Some(result.outcome);
        entry.last_work_count = result.work_count;
        match result.outcome {
            TimerExecutionOutcome::Success | TimerExecutionOutcome::NoWork => {
                entry.last_success_at_ns = Some(now_ns);
                entry.consecutive_expected_failures = 0;
                entry.successes = entry.successes.saturating_add(1);
            }
            TimerExecutionOutcome::RetryableFailure => {
                entry.last_failure_at_ns = Some(now_ns);
                entry.consecutive_expected_failures =
                    entry.consecutive_expected_failures.saturating_add(1);
                entry.expected_failures = entry.expected_failures.saturating_add(1);
            }
            TimerExecutionOutcome::InvariantFailure => {
                entry.last_failure_at_ns = Some(now_ns);
                entry.consecutive_expected_failures = 0;
                entry.invariant_failures = entry.invariant_failures.saturating_add(1);
            }
        }
        let action = entry
            .control
            .complete(generation, directive_deadline_ns)
            .unwrap_or_else(|err| panic!("timer completion failed closed: {err}"));
        if let Some(mode) =
            effective_scheduling_mode(action, directive_deadline_ns, scheduling_mode)
        {
            entry.scheduling_mode = mode;
        }
        entry.condition = match action {
            TimerControlAction::Disarm { cancelled: true } => {
                if entry.enabled {
                    TimerProcessCondition::Idle
                } else {
                    TimerProcessCondition::Disabled
                }
            }
            TimerControlAction::Disarm { cancelled: false } => match result.outcome {
                TimerExecutionOutcome::Success | TimerExecutionOutcome::NoWork => {
                    TimerProcessCondition::Idle
                }
                TimerExecutionOutcome::RetryableFailure
                | TimerExecutionOutcome::InvariantFailure => TimerProcessCondition::Failed,
            },
            TimerControlAction::Arm { .. } => {
                if matches!(result.outcome, TimerExecutionOutcome::InvariantFailure) {
                    TimerProcessCondition::Failed
                } else if matches!(result.outcome, TimerExecutionOutcome::RetryableFailure)
                    || matches!(result.directive, TimerDirective::RetryAfter(_))
                {
                    TimerProcessCondition::Retrying
                } else {
                    TimerProcessCondition::Active
                }
            }
            TimerControlAction::None
            | TimerControlAction::Replace { .. }
            | TimerControlAction::Clear => {
                panic!("timer completion produced an invalid control action")
            }
        };
        let remove =
            !entry.retain_when_stopped && matches!(action, TimerControlAction::Disarm { .. });
        (action, remove)
    });

    if remove {
        TIMERS.with_borrow_mut(|timers| {
            timers.remove(&identity);
        });
    } else {
        apply_action(identity, action);
    }
}

fn effective_scheduling_mode(
    action: TimerControlAction,
    directive_deadline_ns: Option<u64>,
    directive_mode: Option<TimerSchedulingMode>,
) -> Option<TimerSchedulingMode> {
    let TimerControlAction::Arm { deadline_ns, .. } = action else {
        return None;
    };
    Some(if Some(deadline_ns) == directive_deadline_ns {
        directive_mode.unwrap_or(TimerSchedulingMode::Deadline)
    } else {
        TimerSchedulingMode::Deadline
    })
}

fn next_application_timer_id() -> u64 {
    NEXT_APPLICATION_TIMER_ID.with(|next| {
        let id = next
            .get()
            .checked_add(1)
            .unwrap_or_else(|| panic!("application timer identity exhausted"));
        next.set(id);
        id
    })
}

fn deadline_after(now_ns: u64, delay: Duration) -> Result<u64, &'static str> {
    let delay_ns = u64::try_from(delay.as_nanos()).map_err(|_| "timer delay exceeds u64 nanos")?;
    now_ns
        .checked_add(delay_ns)
        .ok_or("timer deadline exceeds u64 nanos")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadline_conversion_is_checked() {
        assert_eq!(deadline_after(10, Duration::from_nanos(5)), Ok(15));
        assert_eq!(
            deadline_after(u64::MAX, Duration::from_nanos(1)),
            Err("timer deadline exceeds u64 nanos")
        );
    }

    #[test]
    fn fixed_timer_labels_are_unique_and_low_cardinality() {
        let keys = [
            TimerKey::AuthRenewal,
            TimerKey::CycleTopup,
            TimerKey::IntentCleanup,
            TimerKey::LogRetention,
            TimerKey::PlacementReceiptAcknowledgement,
            TimerKey::PoolReset,
        ];
        let labels = keys.map(TimerKey::label);
        let unique = labels
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(unique.len(), keys.len());
        assert!(labels.into_iter().all(|label| label.len() <= 64));
    }
}
