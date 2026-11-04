//! Framework namespace grants composed by the single Storage host.

use crate::memory::{CANIC_CONTROL_PLANE_MEMORY_AUTHORITY, CANIC_CORE_MEMORY_AUTHORITY};
use ic_storage::{AdmissionError, AllocationGrants, MemoryAuthority};

/// Compose framework and application namespaces without numeric placement policy.
pub fn framework_grants(
    mut authorities: Vec<MemoryAuthority>,
) -> Result<AllocationGrants, AdmissionError> {
    authorities.push(MemoryAuthority::new(
        CANIC_CORE_MEMORY_AUTHORITY,
        "canic.core.",
    )?);
    authorities.push(MemoryAuthority::new(
        CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
        "canic.control_plane.",
    )?);
    AllocationGrants::new(authorities)
}
