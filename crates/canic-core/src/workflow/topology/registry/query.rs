//! Module: workflow::topology::registry::query
//!
//! Responsibility: expose read-only app and subnet registry workflow snapshots.
//! Does not own: registry storage mutation, endpoint authorization, or DTO schemas.
//! Boundary: workflow query facade over registry storage ops.

use crate::{
    dto::topology::{AppRegistryResponse, SubnetRegistryResponse},
    ops::storage::registry::{
        app::AppRegistryOps, mapper::AppRegistryResponseMapper, subnet::SubnetRegistryOps,
    },
};

///
/// AppRegistryQuery
///

pub struct AppRegistryQuery;

impl AppRegistryQuery {
    pub fn registry() -> AppRegistryResponse {
        let data = AppRegistryOps::data();

        AppRegistryResponseMapper::data_to_response(data)
    }
}

///
/// SubnetRegistryQuery
///

pub struct SubnetRegistryQuery;

impl SubnetRegistryQuery {
    pub fn registry() -> SubnetRegistryResponse {
        SubnetRegistryOps::response()
    }
}
