//! App mode gating for endpoints.
//!
//! The app bucket only inspects the application mode and does not
//! evaluate caller identity or environment predicates.

use crate::{access::AccessError, ops::storage::state::app::AppStateOps};

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
