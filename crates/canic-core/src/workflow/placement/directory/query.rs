//! Module: workflow::placement::directory::query
//!
//! Responsibility: expose read-only directory registry query projections.
//! Does not own: directory mutation, child lifecycle, or endpoint authorization.
//! Boundary: delegates storage reads and maps them into directory DTO responses.

use crate::{
    cdk::types::Principal,
    dto::placement::directory::{DirectoryEntryStatusResponse, DirectoryRegistryResponse},
    ops::storage::placement::directory::DirectoryRegistryOps,
};

///
/// DirectoryQuery
///
/// Read-only query facade for directory registry state.
///

pub struct DirectoryQuery;

impl DirectoryQuery {
    #[must_use]
    pub fn lookup_key(pool: &str, key_value: &str) -> Option<Principal> {
        DirectoryRegistryOps::lookup_key(pool, key_value)
    }

    #[must_use]
    pub fn lookup_entry(pool: &str, key_value: &str) -> Option<DirectoryEntryStatusResponse> {
        DirectoryRegistryOps::lookup_entry(pool, key_value)
    }

    #[must_use]
    pub fn registry() -> DirectoryRegistryResponse {
        DirectoryRegistryOps::entries_response()
    }
}
