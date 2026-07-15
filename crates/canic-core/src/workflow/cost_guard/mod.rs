//! Module: workflow::cost_guard
//!
//! Responsibility: map cost guard ops errors onto workflow/public error surfaces.
//! Does not own: quota accounting, cycle reservations, or command policy.
//! Boundary: workflow modules call this after `CostGuardOps::reserve` failures.

use crate::{
    InternalError,
    dto::error::Error,
    ops::cost_guard::{CostGuardReserveError, CostGuardReservePublicKind},
};

#[must_use]
pub fn map_cost_guard_reserve_error(err: CostGuardReserveError) -> InternalError {
    match err.public_kind() {
        Some(CostGuardReservePublicKind::InvalidInput) => {
            InternalError::public(Error::invalid(err.to_string()))
        }
        Some(CostGuardReservePublicKind::ResourceExhausted) => {
            InternalError::public(Error::exhausted(err.to_string()))
        }
        None => err.into(),
    }
}
