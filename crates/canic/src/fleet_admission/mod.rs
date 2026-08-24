//! Synchronous Fleet-admission guard for endpoints owned by a composed framework.
//!
//! This facade reads the observed transport caller and Canic's existing managed
//! projection. It owns no storage, lifecycle work, timer, or remote lookup.

use crate::access::AccessError;
use candid::Principal;

/// Return the observed transport caller when the exact local projection admits it.
///
/// Missing, invalid, stale, or fenced projection authority fails closed.
pub fn require_caller() -> Result<Principal, AccessError> {
    crate::__internal::core::access::auth::require_fleet_admitted_caller()
}
