//! Module: workflow::topology::registry::query
//!
//! Responsibility: expose read-only subnet registry workflow snapshots.
//! Does not own: registry storage mutation, endpoint authorization, or DTO schemas.
//! Boundary: workflow query facade over registry storage ops.

use crate::{
    dto::topology::SubnetRegistryResponse, ops::storage::registry::subnet::SubnetRegistryOps,
};

///
/// SubnetRegistryQuery
///

pub struct SubnetRegistryQuery;

impl SubnetRegistryQuery {
    pub fn registry() -> SubnetRegistryResponse {
        SubnetRegistryOps::response()
    }
}
