//! Module: access::app
//!
//! Responsibility: gate endpoint access by application mode.
//! Does not own: caller identity, environment predicates, or endpoint error mapping.
//! Boundary: access expressions call this before endpoint workflow execution.

use crate::{access::AccessError, ops::storage::state::app::AppStateOps};

///
/// guard_app_query
///
/// Validate access for query calls.
///
/// Behavior:
/// - Enabled and Readonly modes permit queries.
/// - Disabled mode rejects queries.
pub fn guard_app_query() -> Result<(), AccessError> {
    if AppStateOps::is_query_allowed() {
        Ok(())
    } else {
        Err(AccessError::Denied("application is disabled".to_string()))
    }
}

///
/// guard_app_update
///
/// Validate access for update calls.
///
/// Behavior:
/// - Enabled mode permits updates.
/// - Readonly rejects updates.
/// - Disabled rejects updates.
pub fn guard_app_update() -> Result<(), AccessError> {
    if AppStateOps::is_update_allowed() {
        return Ok(());
    }

    if AppStateOps::is_readonly() {
        Err(AccessError::Denied(
            "application is in readonly mode".to_string(),
        ))
    } else {
        Err(AccessError::Denied("application is disabled".to_string()))
    }
}
