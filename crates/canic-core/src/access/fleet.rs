//! Module: access::fleet
//!
//! Responsibility: gate endpoint access by Fleet mode.
//! Does not own: caller identity, environment predicates, or endpoint error mapping.
//! Boundary: access expressions call this before endpoint workflow execution.

use crate::{access::AccessError, ops::storage::state::fleet::FleetStateOps};

///
/// guard_fleet_query
///
/// Validate access for query calls.
///
/// Behavior:
/// - Enabled and Readonly modes permit queries.
/// - Disabled mode rejects queries.
pub fn guard_fleet_query() -> Result<(), AccessError> {
    if FleetStateOps::is_query_allowed() {
        Ok(())
    } else {
        Err(AccessError::Denied("Fleet is disabled".to_string()))
    }
}

///
/// guard_fleet_update
///
/// Validate access for update calls.
///
/// Behavior:
/// - Enabled mode permits updates.
/// - Readonly rejects updates.
/// - Disabled rejects updates.
pub fn guard_fleet_update() -> Result<(), AccessError> {
    if FleetStateOps::is_update_allowed() {
        return Ok(());
    }

    if FleetStateOps::is_readonly() {
        Err(AccessError::Denied("Fleet is in readonly mode".to_string()))
    } else {
        Err(AccessError::Denied("Fleet is disabled".to_string()))
    }
}
