//! Module: access::caller_authority
//!
//! Local managed-caller admission at authenticated endpoint and subsequent effect boundaries.

use crate::{
    access::AccessError,
    ops::{caller_authority::CallerAuthorityOps, config::ConfigOps, ic::IcOps},
};

pub use crate::domain::policy::pure::caller_authority::{
    CallerAdmissionError, CallerAdmissionTicket, CallerTargetTicket,
};

/// Admit the IC transport caller for a named compiled permission without a remote lookup.
pub fn admit(permission: &str) -> Result<CallerAdmissionTicket, AccessError> {
    ConfigOps::with_caller_policy(|policy| {
        CallerAuthorityOps::admit(IcOps::msg_caller(), permission, policy)
    })
    .map_err(AccessError::Internal)?
    .map_err(AccessError::CallerAuthority)
}

/// Require the original ticket to remain valid before issuing the next new protected effect.
pub fn revalidate(ticket: &CallerAdmissionTicket) -> Result<(), AccessError> {
    ConfigOps::with_caller_policy(|policy| CallerAuthorityOps::revalidate(ticket, policy))
        .map_err(AccessError::Internal)?
        .map_err(AccessError::CallerAuthority)
}

/// Select a permitted proxy destination; separately authenticate the incoming application subject.
pub fn select_target(
    target: candid::Principal,
    permission: &str,
) -> Result<CallerTargetTicket, AccessError> {
    ConfigOps::with_caller_policy(|policy| {
        CallerAuthorityOps::select_target(target, permission, policy)
    })
    .map_err(AccessError::Internal)?
    .map_err(AccessError::CallerAuthority)
}

/// Recheck the retained destination before issuing a new remote effect after an await.
pub fn revalidate_target(ticket: &CallerTargetTicket) -> Result<(), AccessError> {
    ConfigOps::with_caller_policy(|policy| CallerAuthorityOps::revalidate_target(ticket, policy))
        .map_err(AccessError::Internal)?
        .map_err(AccessError::CallerAuthority)
}
