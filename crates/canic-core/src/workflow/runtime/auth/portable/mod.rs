//! Module: workflow::runtime::auth::portable
//!
//! Responsibility: schedule receiver enrollment and recover exact abandoned refresh attempts.
//! Boundary: native Timers owns execution; ops owns receiver state and all projections.

use crate::{
    InternalError,
    ops::{
        auth::portable::{self, runtime},
        ic::IcOps,
        storage::async_job_recovery::AsyncJobOwner,
    },
    workflow::runtime::{
        async_job::AsyncJobWorkflow,
        timer::{self, TimerAuthorityWorkflow, TimerError},
    },
};
use ic_timers::{
    OnceDecision, OnceRegistration, OnceRunResult, TimerCompletion, TimerIdentity, TimerSchedule,
    reconcile_once,
};
use std::cell::RefCell;

thread_local! { static REFRESH: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) }; }

pub struct PortableSessionWorkflow;

impl PortableSessionWorkflow {
    pub(crate) fn inspect_policy(
        now_ns: u64,
    ) -> Result<
        canic_contracts::dto::portable_inspection::PolicyReport,
        canic_contracts::dto::portable_inspection::Refusal,
    > {
        runtime::inspection_policy(now_ns)
    }

    pub(crate) fn inspect(
        request: canic_contracts::dto::portable_inspection::Request,
        now_ns: u64,
    ) -> Result<
        canic_contracts::dto::portable_inspection::Page,
        canic_contracts::dto::portable_inspection::Refusal,
    > {
        crate::ops::storage::auth::LocalApplicationAuthorizationStateOps::inspect_application_sessions(request, now_ns)
    }

    pub(crate) fn establish(
        caller: ic_platform::Principal,
        now_ns: u64,
        input: ic_auth_wire::SessionInput,
    ) -> Result<ic_auth_wire::SessionView, portable::PortableSessionError> {
        runtime::admit(caller, now_ns, input)
    }

    pub(crate) fn read(
        caller: ic_platform::Principal,
        now_ns: u64,
        scopes: Vec<String>,
        replicated: bool,
    ) -> Result<ic_auth_wire::SessionView, portable::PortableSessionError> {
        runtime::read(caller, now_ns, scopes, replicated)
    }

    pub(crate) fn logout(
        caller: ic_platform::Principal,
        now_ns: u64,
    ) -> Result<ic_auth_protocol_types::LogoutResult, portable::PortableSessionError> {
        runtime::logout(caller, now_ns)
    }

    pub fn identity() -> Result<TimerIdentity, TimerError> {
        TimerIdentity::try_new("canic", "portable_enrollment", "refresh").map_err(Into::into)
    }

    pub fn claimed_identity() -> Result<Option<TimerIdentity>, TimerError> {
        timer::owned_once_identity(&REFRESH)
    }

    pub fn cancel() -> Result<(), TimerError> {
        timer::cancel_owned_once(&REFRESH)
    }

    pub fn start() -> Result<(), InternalError> {
        if runtime::profile().is_none() {
            return Ok(());
        }
        Self::schedule(IcOps::now_nanos())?;
        TimerAuthorityWorkflow::ensure_async_job_recovery_watchdog()?;
        Ok(())
    }

    fn schedule(deadline: u64) -> Result<(), TimerError> {
        timer::require_active()?;
        REFRESH.with(|slot| {
            let mut registration = slot.try_borrow_mut().map_err(|_| TimerError::CustodyBusy)?;
            reconcile_once(
                &mut registration,
                &Self::identity()?,
                Some(TimerSchedule::At(deadline)),
                |_| Self::run(),
            )
            .map_err(Into::into)
        })
    }

    async fn refresh() -> Result<(), portable::PortableSessionError> {
        runtime::maintain(IcOps::now_nanos())?;
        if let Some(ticket) = runtime::prepare_boot()? {
            let challenge = crate::ops::auth::enrollment::fresh_challenge().await?;
            runtime::finish_boot(ticket, challenge)?;
        }
        let pull = runtime::prepare_refresh(IcOps::now_nanos())?;
        let reply = pull.fetch().await?;
        runtime::finish_refresh(reply)
    }

    async fn run() -> OnceRunResult {
        let attempt = match AsyncJobWorkflow::claim(AsyncJobOwner::PortableEnrollment) {
            Ok(attempt) => attempt,
            Err(result) => return result,
        };
        let result = Self::refresh().await;
        if let Err(error) = &result {
            crate::log!(
                crate::log::Topic::Auth,
                Warn,
                "portable enrollment refresh refused: {error}"
            );
        }
        let next = runtime::profile().and_then(|profile| {
            IcOps::now_nanos().checked_add(profile.enrollment_refresh_interval_ns)
        });
        let decision = next.map_or(OnceDecision::Stop, OnceDecision::ScheduleAt);
        let completion = if result.is_ok() {
            TimerCompletion::success(1)
        } else {
            TimerCompletion::retryable_failure(0)
        };
        AsyncJobWorkflow::finish(attempt, OnceRunResult::new(completion, decision))
    }

    pub fn recover_expired(now_ns: u64) -> bool {
        if timer::require_active().is_err() {
            return false;
        }
        let owner = AsyncJobOwner::PortableEnrollment;
        if runtime::profile().is_none() {
            return AsyncJobWorkflow::abandon_expired(owner, now_ns);
        }
        let Ok(identity) = Self::identity() else {
            return false;
        };
        if !TimerAuthorityWorkflow::registration_requires_recovery(
            &identity,
            AsyncJobWorkflow::has_expired_attempt(owner, now_ns),
        )
        .unwrap_or(false)
        {
            return false;
        }
        Self::schedule(now_ns).is_ok()
    }
}
