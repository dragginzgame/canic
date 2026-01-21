//! App mode gating for endpoints.
//!
//! The guard bucket only inspects the application mode and does not
//! evaluate caller identity or environment predicates.

use crate::{
    access::AccessError, ops::storage::state::app::AppStateOps,
    storage::stable::state::app::AppMode,
};

/// Validate access for query calls.
///
/// Behavior:
/// - Enabled and Readonly modes permit queries.
/// - Disabled mode rejects queries.
pub fn guard_app_query() -> Result<(), AccessError> {
    let mode = AppStateOps::get_mode();

    match mode {
        AppMode::Enabled | AppMode::Readonly => Ok(()),
        AppMode::Disabled => Err(AccessError::Denied("application is disabled".to_string())),
    }
}

/// Validate access for update calls.
///
/// Behavior:
/// - Enabled mode permits updates.
/// - Readonly rejects updates.
/// - Disabled rejects updates.
pub fn guard_app_update() -> Result<(), AccessError> {
    let mode = AppStateOps::get_mode();

    match mode {
        AppMode::Enabled => Ok(()),
        AppMode::Readonly => Err(AccessError::Denied(
            "application is in readonly mode".to_string(),
        )),
        AppMode::Disabled => Err(AccessError::Denied("application is disabled".to_string())),
    }
}
