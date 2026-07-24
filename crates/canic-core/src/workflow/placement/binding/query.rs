//! Module: workflow::placement::binding::query
//!
//! Responsibility: expose read-only binding registry query projections.
//! Does not own: binding mutation, child lifecycle, or endpoint authorization.
//! Boundary: delegates storage reads and maps them into binding DTO responses.

use crate::{
    cdk::types::Principal,
    dto::placement::binding::{PlacementBindingRegistryResponse, PlacementBindingStatusResponse},
    ops::storage::placement::binding::PlacementBindingRegistryOps,
};

///
/// PlacementBindingQuery
///
/// Read-only query facade for binding registry state.
///

pub struct PlacementBindingQuery;

impl PlacementBindingQuery {
    #[must_use]
    pub fn lookup_key(pool: &str, key_value: &str) -> Option<Principal> {
        PlacementBindingRegistryOps::lookup_key(pool, key_value)
    }

    #[must_use]
    pub fn lookup_entry(pool: &str, key_value: &str) -> Option<PlacementBindingStatusResponse> {
        PlacementBindingRegistryOps::lookup_entry(pool, key_value)
    }

    #[must_use]
    pub fn registry() -> PlacementBindingRegistryResponse {
        PlacementBindingRegistryOps::entries_response()
    }
}
