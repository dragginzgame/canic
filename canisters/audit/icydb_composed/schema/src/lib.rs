//! Empty journaled IcyDB schema, using Canic's existing participant memory range.

use icydb::model::prelude::*;

/// Lifecycle metadata for the empty participant; contains no entity registrations.
#[canister(memory_namespace = "canic_composed_audit")]
pub struct EmptyCanister {}

/// One empty journaled store matching the existing IcyDB audit shape.
#[store(canister = "EmptyCanister", storage(journaled(key = "empty")))]
pub struct EmptyStore {}
