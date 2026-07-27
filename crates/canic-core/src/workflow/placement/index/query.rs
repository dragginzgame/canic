//! Module: workflow::placement::index::query
//!
//! Responsibility: expose read-only index registry query projections.
//! Does not own: index mutation, child lifecycle, or endpoint authorization.
//! Boundary: delegates storage reads and maps them into index DTO responses.

use crate::{
    cdk::types::Principal,
    dto::placement::index::{PlacementIndexRegistryResponse, PlacementIndexStatusResponse},
    ops::storage::placement::index::PlacementIndexRegistryOps,
};

///
/// PlacementIndexQuery
///
/// Read-only query facade for index registry state.
///

pub struct PlacementIndexQuery;

impl PlacementIndexQuery {
    #[must_use]
    pub fn lookup_key(pool: &str, key_value: &str) -> Option<Principal> {
        PlacementIndexRegistryOps::lookup_key(pool, key_value)
    }

    #[must_use]
    pub fn lookup_entry(pool: &str, key_value: &str) -> Option<PlacementIndexStatusResponse> {
        PlacementIndexRegistryOps::lookup_entry(pool, key_value)
    }

    #[must_use]
    pub fn registry() -> PlacementIndexRegistryResponse {
        PlacementIndexRegistryOps::entries_response()
    }
}
