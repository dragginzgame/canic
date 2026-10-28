//! Module: ops::storage::auth::application_sessions::receiver
//!
//! Responsibility: retain one persistent receiver ledger and independent work budgets.
//! Boundary: the host supplies protected policy, fresh enrollment, runtime caller and time.
//! This component installs no endpoints and does not activate application authorization.

use crate::{
    model::auth::application_authorization::{
        MAX_APPLICATION_ADMITTED_LOGINS, MAX_APPLICATION_UNKNOWN_LOGOUTS,
    },
    ops::storage::auth::{
        LocalApplicationAuthorizationStateOps,
        application_sessions::{
            ApplicationSessionCleanupResult, ApplicationSessionStateError, ApplicationSessionStore,
            invalidate_indexes, prepare_maintenance_state, with_indexes,
        },
    },
    storage::stable::auth::{
        LocalApplicationAuthorityBindingRecord, LocalApplicationAuthorizationState,
    },
};
use canic_contracts::ids::FleetKey;
use ic_auth::{
    admission_budget::SessionAdmissionBudget,
    portable::{LoginRevocationChange, MemoryLoginRevocations},
    session::SessionError,
};
use ic_auth_protocol_types::{AudienceId, CanonicalId};
use std::cell::Cell;

thread_local! {
    static RUNTIME_GENERATION: Cell<u64> = const { Cell::new(0) };
}

pub(super) fn invalidate_runtime() {
    RUNTIME_GENERATION.with(|generation| {
        generation.set(
            generation
                .get()
                .checked_add(1)
                .expect("receiver runtime generation"),
        );
    });
}

/// Protected host selection, independent of issuer mechanism and key custody.
/// Clock skew requires deployment qualification; this API supplies no default.
#[derive(Clone, Copy, Debug)]
pub struct PersistentReceiverPolicy {
    pub fleet: FleetKey,
    pub clock_skew_ns: u64,
    pub verification_burst: u32,
    pub verification_refill_interval_ns: u64,
    pub unknown_logout_burst: u32,
    pub unknown_logout_refill_interval_ns: u64,
}

/// One host-retained receiver, reconstructed synchronously after stable restoration.
///
/// Keep it across callers and proof mechanisms. Restoring another owner or running
/// external storage maintenance fences the previous owner. Fresh trusted enrollment
/// is still required for every admission and authorization context supplied to Auth.
pub struct PersistentReceiverState {
    store: ApplicationSessionStore,
    ledger: MemoryLoginRevocations,
    verification: SessionAdmissionBudget,
    unknown_logout: SessionAdmissionBudget,
    runtime_generation: u64,
}

impl PersistentReceiverState {
    /// Restore the complete physical ledger, including expired records and denials.
    /// A refusal closes derived access. Neither restoration nor budget reset renews
    /// a login; both independent budgets start empty at the current trusted time.
    pub fn restore(policy: PersistentReceiverPolicy, now_ns: u64) -> Result<Self, SessionError> {
        invalidate_indexes();
        // Capture the old high-water mark before startup observes current time.
        // The Auth restore validator checks records against their own checkpoint.
        let checkpoint_time = LocalApplicationAuthorizationState::application_observed_at_ns();
        if !LocalApplicationAuthorizationState::observe_application_time(now_ns) {
            return Err(SessionError::AdmissionClockRollback);
        }
        let verification = SessionAdmissionBudget::new(
            policy.verification_burst,
            policy.verification_refill_interval_ns,
            now_ns,
        )?;
        let unknown_logout = SessionAdmissionBudget::new(
            policy.unknown_logout_burst,
            policy.unknown_logout_refill_interval_ns,
            now_ns,
        )?;
        let restore = || -> Result<MemoryLoginRevocations, SessionError> {
            let (checkpoint, _) = prepare_maintenance_state(now_ns).map_err(map_state_error)?;
            if let Some(LocalApplicationAuthorityBindingRecord::Enabled { fleet, .. }) =
                &checkpoint.authority_binding
                && *fleet != policy.fleet
            {
                return Err(SessionError::InvalidPolicyOrRecord);
            }
            let audience = AudienceId {
                canonical_network_id: CanonicalId::from_bytes(
                    *policy.fleet.canonical_network_id.as_bytes(),
                ),
                fleet_id: CanonicalId::from_bytes(*policy.fleet.fleet_id.as_bytes()),
            };
            let records = checkpoint
                .logins
                .iter()
                .map(|(_, record)| LoginRevocationChange::decode(&record.bytes))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(MemoryLoginRevocations::new(
                audience,
                MAX_APPLICATION_ADMITTED_LOGINS,
                MAX_APPLICATION_UNKNOWN_LOGOUTS,
                policy.clock_skew_ns,
                now_ns,
            )?
            .restore_records(
                checkpoint_time,
                (
                    checkpoint.login_totals.admitted,
                    checkpoint.login_totals.unknown,
                ),
                records,
            )?)
        };
        let ledger = match restore() {
            Ok(ledger) => ledger,
            Err(error) => {
                invalidate_indexes();
                return Err(error);
            }
        };
        Ok(Self {
            store: ApplicationSessionStore {
                fleet: policy.fleet,
            },
            ledger,
            verification,
            unknown_logout,
            runtime_generation: RUNTIME_GENERATION.with(Cell::get),
        })
    }

    /// Serialize one complete Auth operation against the retained host state.
    /// The host acquires caller/time. Admission and authorization contexts require
    /// fresh receiver-bound enrollment; logout needs restored state and the runtime
    /// caller even after enrollment expires. Never pass client-selected authority.
    /// No await, reentrancy, restoration or external maintenance may enter the closure.
    /// Returned errors retain charged budgets and the protected clock high-water mark.
    pub fn run<T>(
        &mut self,
        now_ns: u64,
        operation: impl FnOnce(
            &mut ApplicationSessionStore,
            &mut MemoryLoginRevocations,
            &mut SessionAdmissionBudget,
            &mut SessionAdmissionBudget,
        ) -> Result<T, SessionError>,
    ) -> Result<T, SessionError> {
        self.check_current()?;
        if !LocalApplicationAuthorizationState::observe_application_time(now_ns) {
            invalidate_indexes();
            return Err(SessionError::AdmissionClockRollback);
        }
        let result = operation(
            &mut self.store,
            &mut self.ledger,
            &mut self.verification,
            &mut self.unknown_logout,
        );
        // Retain clock advancement even when a checked engine operation refuses.
        let observed = now_ns.max(self.ledger.observed_at_ns());
        if !LocalApplicationAuthorizationState::observe_application_time(observed) {
            invalidate_indexes();
            return Err(SessionError::AdmissionClockRollback);
        }
        self.check_current()?;
        result
    }

    /// Run bounded session/replay cleanup while keeping this ledger owner current.
    /// Use this method rather than invoking external maintenance on a live owner.
    pub fn cleanup_sessions(
        &mut self,
        now_ns: u64,
    ) -> Result<ApplicationSessionCleanupResult, SessionError> {
        self.check_current()?;
        let result = LocalApplicationAuthorizationStateOps::cleanup_application_sessions(now_ns)
            .map_err(map_state_error)?;
        self.runtime_generation = RUNTIME_GENERATION.with(Cell::get);
        Ok(result)
    }

    /// Prune logout rows through the retained ledger after bounded session cleanup.
    /// A failed prune fences this owner; restoration is required before further use.
    pub fn prune_logins(&mut self, now_ns: u64, budget: usize) -> Result<usize, SessionError> {
        self.check_current()?;
        let removed = self.store.prune_logins(&mut self.ledger, now_ns, budget)?;
        self.runtime_generation = RUNTIME_GENERATION.with(Cell::get);
        Ok(removed)
    }

    fn check_current(&self) -> Result<(), SessionError> {
        if self.runtime_generation != RUNTIME_GENERATION.with(Cell::get) {
            return Err(SessionError::StorageFailure);
        }
        with_indexes(|_| ()).map_err(map_state_error)
    }
}

const fn map_state_error(error: ApplicationSessionStateError) -> SessionError {
    match error {
        ApplicationSessionStateError::ClockRollback => SessionError::AdmissionClockRollback,
        _ => SessionError::StorageFailure,
    }
}
