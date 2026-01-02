use crate::{PublicError, cdk::types::Principal, ids::CanisterRole, workflow::children::query};

/// Lookup the first direct child matching the role in the children cache.
///
/// Returns `None` when no matching child is cached.
pub fn canic_child_by_role(role: CanisterRole) -> Result<Option<Principal>, PublicError> {
    query::child_pid_by_role(role).map_err(PublicError::from)
}
