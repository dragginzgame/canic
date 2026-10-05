//! Compose indexed receiver reads with pure admission decisions.

use crate::{
    config::caller_authority::CompiledCallerPolicy,
    domain::policy::pure::caller_authority::{self, CallerAdmissionTicket, CallerTargetTicket},
    model::caller_authority::CallerAdmissionError,
    ops::caller_authority::CallerAuthorityOps,
};
use candid::Principal;

/// Local admission orchestration; endpoint callers supply the authenticated transport identity.
pub struct CallerAdmissionWorkflow;

impl CallerAdmissionWorkflow {
    /// Local transport-caller admission with constant indexed source and Component-fence lookups.
    pub fn require_permission(
        caller: Principal,
        permission: &str,
        policy: &CompiledCallerPolicy,
    ) -> Result<(), CallerAdmissionError> {
        CallerAuthorityOps::with_admission(caller, |view| {
            caller_authority::require_permission(caller, permission, policy, view)
        })
    }

    /// Retain exact local authority when the application will continue across an await.
    pub fn admit(
        caller: Principal,
        permission: &str,
        policy: &CompiledCallerPolicy,
    ) -> Result<CallerAdmissionTicket, CallerAdmissionError> {
        CallerAuthorityOps::with_admission(caller, |view| {
            caller_authority::admit(caller, permission, policy, view)
        })
    }

    /// Revalidate retained endpoint authority before issuing a new effect after an await.
    pub fn revalidate(
        ticket: &CallerAdmissionTicket,
        policy: &CompiledCallerPolicy,
    ) -> Result<(), CallerAdmissionError> {
        CallerAuthorityOps::with_admission(ticket.caller(), |view| {
            caller_authority::revalidate(ticket, policy, view)
        })
    }

    /// Select only a locally projected target under a target-direction permission.
    pub fn select_target(
        target: Principal,
        permission: &str,
        policy: &CompiledCallerPolicy,
    ) -> Result<CallerTargetTicket, CallerAdmissionError> {
        CallerAuthorityOps::with_admission(target, |view| {
            caller_authority::select_target(target, permission, policy, view)
        })
    }

    /// Preserve target direction when revalidating a retained proxy destination.
    pub fn revalidate_target(
        ticket: &CallerTargetTicket,
        policy: &CompiledCallerPolicy,
    ) -> Result<(), CallerAdmissionError> {
        CallerAuthorityOps::with_admission(ticket.target(), |view| {
            caller_authority::revalidate_target(ticket, policy, view)
        })
    }
}
