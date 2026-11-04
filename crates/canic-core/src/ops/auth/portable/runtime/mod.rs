//! Module: ops::auth::portable::runtime
//!
//! Responsibility: retain the one receiver owner and prepare/finish authenticated refreshes.
//! Boundary: workflow owns scheduling/awaits; no mutable receiver borrow escapes an operation.

mod inspection;
pub use inspection::inspection_policy;

use crate::{
    config::schema::{LocalApplicationAuthorizationConfig, PortableReceiverConfig},
    model::auth::application_authorization::LocalApplicationAuthoritySnapshot,
    ops::{
        auth::{
            AuthOps,
            enrollment::{
                AuthenticatedPortableEnrollment, PortableEnrollmentPull, ReceiverPortableEnrollment,
            },
            portable::{self, PortableReceiverContext, PortableSessionError},
        },
        config::ConfigOps,
        storage::auth::{
            LocalApplicationAuthorizationStateOps,
            application_sessions::{PersistentReceiverPolicy, PersistentReceiverState},
        },
    },
};
use ic_auth_protocol_types::Principal;
use ic_auth_wire::{SessionInput, SessionView};
use std::cell::{Cell, RefCell};

struct Runtime {
    owner: PersistentReceiverState,
    gate: Option<ReceiverPortableEnrollment>,
    identity: LocalApplicationAuthoritySnapshot,
    profile: PortableReceiverConfig,
    epoch: u64,
}

thread_local! {
    static STATE: RefCell<Option<Runtime>> = const { RefCell::new(None) };
    static EPOCH: Cell<u64> = const { Cell::new(0) };
}

/// Exact volatile activation binding retained while the randomness request waits.
pub struct BootTicket {
    epoch: u64,
}

fn current() -> Result<
    (
        LocalApplicationAuthorizationConfig,
        LocalApplicationAuthoritySnapshot,
    ),
    PortableSessionError,
> {
    let authority = AuthOps::local_application_authorization_authority()?
        .ok_or(PortableSessionError::Unavailable)?;
    if authority.config.receiver.is_none() {
        return Err(PortableSessionError::Unavailable);
    }
    require_qualified_clock(ConfigOps::delegated_tokens_config()?.build_network)?;
    Ok((authority.config, authority.snapshot))
}

fn check_binding(
    runtime: &Runtime,
    config: &LocalApplicationAuthorizationConfig,
    identity: &LocalApplicationAuthoritySnapshot,
) -> Result<(), PortableSessionError> {
    if &runtime.identity != identity || config.receiver.as_ref() != Some(&runtime.profile) {
        return Err(PortableSessionError::Unavailable);
    }
    Ok(())
}

/// Restore closed after framework storage/identity reconciliation. No timer or IC call.
pub fn restore(now_ns: u64) -> Result<(), PortableSessionError> {
    STATE.with_borrow_mut(|state| *state = None);
    let Some(authority) = AuthOps::local_application_authorization_authority()? else {
        return Ok(());
    };
    let Some(profile) = authority.config.receiver else {
        return Ok(());
    };
    require_qualified_clock(ConfigOps::delegated_tokens_config()?.build_network)?;
    LocalApplicationAuthorizationStateOps::initialize_receiver_floors(
        now_ns,
        profile.clock_skew_ns,
    )
    .map_err(|_| PortableSessionError::Unavailable)?;
    let owner = PersistentReceiverState::restore(
        PersistentReceiverPolicy {
            max_sessions: profile.max_sessions,
            fleet: authority.snapshot.fleet(),
            clock_skew_ns: profile.clock_skew_ns,
            verification_burst: profile.verification_burst,
            verification_refill_interval_ns: profile.verification_refill_interval_ns,
            unknown_logout_burst: profile.unknown_logout_burst,
            unknown_logout_refill_interval_ns: profile.unknown_logout_refill_interval_ns,
        },
        now_ns,
    )?;
    let epoch = EPOCH.with(|e| {
        let next = e
            .get()
            .checked_add(1)
            .ok_or(PortableSessionError::Unavailable)?;
        e.set(next);
        Ok::<_, PortableSessionError>(next)
    })?;
    STATE.with_borrow_mut(|state| {
        *state = Some(Runtime {
            owner,
            gate: None,
            identity: authority.snapshot,
            profile,
            epoch,
        });
    });
    Ok(())
}

pub fn profile() -> Option<PortableReceiverConfig> {
    STATE.with_borrow(|state| state.as_ref().map(|s| s.profile.clone()))
}

pub fn prepare_boot() -> Result<Option<BootTicket>, PortableSessionError> {
    let (config, identity) = current()?;
    STATE.with_borrow(|state| {
        let state = state.as_ref().ok_or(PortableSessionError::Unavailable)?;
        check_binding(state, &config, &identity)?;
        Ok(state
            .gate
            .is_none()
            .then_some(BootTicket { epoch: state.epoch }))
    })
}

pub fn finish_boot(ticket: BootTicket, challenge: [u8; 32]) -> Result<(), PortableSessionError> {
    let (config, identity) = current()?;
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().ok_or(PortableSessionError::Unavailable)?;
        check_binding(state, &config, &identity)?;
        if state.epoch != ticket.epoch {
            return Err(PortableSessionError::Unavailable);
        }
        if state.gate.is_none() {
            state.gate = Some(ReceiverPortableEnrollment::boot(challenge)?);
        }
        Ok(())
    })
}

pub fn prepare_refresh(now_ns: u64) -> Result<PortableEnrollmentPull, PortableSessionError> {
    let (config, identity) = current()?;
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().ok_or(PortableSessionError::Unavailable)?;
        check_binding(state, &config, &identity)?;
        let deadline = now_ns
            .checked_add(state.profile.enrollment_receipt_timeout_ns)
            .ok_or(PortableSessionError::Unavailable)?;
        Ok(state
            .gate
            .as_mut()
            .ok_or(PortableSessionError::Unavailable)?
            .prepare_pull(deadline)?)
    })
}

pub fn finish_refresh(reply: AuthenticatedPortableEnrollment) -> Result<(), PortableSessionError> {
    let (config, identity) = current()?;
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().ok_or(PortableSessionError::Unavailable)?;
        check_binding(state, &config, &identity)?;
        state
            .gate
            .as_mut()
            .ok_or(PortableSessionError::Unavailable)?
            .finish_pull(reply)?;
        Ok(())
    })
}

fn with_context<T>(
    caller: Principal,
    now_ns: u64,
    replicated: bool,
    operation: impl FnOnce(
        &mut PersistentReceiverState,
        PortableReceiverContext<'_>,
    ) -> Result<T, PortableSessionError>,
) -> Result<T, PortableSessionError> {
    let (config, identity) = current()?;
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().ok_or(PortableSessionError::Unavailable)?;
        check_binding(state, &config, &identity)?;
        if replicated {
            state.owner.observe(now_ns)?;
        }
        let snapshot = state
            .gate
            .as_ref()
            .ok_or(PortableSessionError::Unavailable)?
            .current()?;
        operation(
            &mut state.owner,
            PortableReceiverContext {
                config: &config,
                identity: &identity,
                snapshot,
                caller,
                now_ns,
            },
        )
    })
}

pub fn admit(
    caller: Principal,
    now_ns: u64,
    input: SessionInput,
) -> Result<SessionView, PortableSessionError> {
    with_context(caller, now_ns, true, |owner, context| {
        portable::admit(owner, context, input)
    })
}

pub fn read(
    caller: Principal,
    now_ns: u64,
    scopes: Vec<String>,
    replicated: bool,
) -> Result<SessionView, PortableSessionError> {
    with_context(caller, now_ns, replicated, |owner, context| {
        portable::read(owner, context, scopes, replicated)
    })
}

/// Logout requires retained state and the actual caller, independently of enrollment.
pub fn logout(
    caller: Principal,
    now_ns: u64,
) -> Result<ic_auth_protocol_types::LogoutResult, PortableSessionError> {
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().ok_or(PortableSessionError::Unavailable)?;
        portable::logout(&mut state.owner, caller, now_ns)
    })
}

/// Bound physical reclamation independently of enrollment availability.
pub fn maintain(now_ns: u64) -> Result<(), PortableSessionError> {
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().ok_or(PortableSessionError::Unavailable)?;
        let start = crate::perf::perf_counter();
        let cleanup = state.owner.cleanup_sessions(now_ns)?;
        crate::perf::record_checkpoint(
            module_path!(),
            "application_session_cleanup",
            crate::perf::perf_counter().saturating_sub(start),
        );
        crate::ops::runtime::metrics::auth::record_application_session_cleanup(
            cleanup.total_removed(),
        );
        // Finish session/replay cleanup before Auth can prune a login reservation.
        // A full batch continues on the next scheduled refresh, without an unbounded loop.
        let due = LocalApplicationAuthorizationStateOps::application_session_cleanup_due_at_ns()
            .map_err(|_| PortableSessionError::Unavailable)?;
        if due.is_none_or(|deadline| deadline > now_ns) {
            state.owner.prune_logins(now_ns, 128)?;
        }
        Ok(())
    })
}

// The maintainer-selected production clock bound is still unqualified. Local
// controlled-clock qualification does not turn its fixture value into IC policy.
pub(super) const fn require_qualified_clock(
    network: canic_contracts::ids::BuildNetwork,
) -> Result<(), PortableSessionError> {
    if crate::domain::policy::pure::auth::portable_clock_is_qualified(network) {
        Ok(())
    } else {
        Err(PortableSessionError::UnqualifiedClock)
    }
}
