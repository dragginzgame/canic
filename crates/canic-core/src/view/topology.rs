//! Module: view::topology
//!
//! Responsibility: expose internal read-only topology projections.
//! Does not own: stable records, endpoint DTOs, or topology decisions.
//! Boundary: storage ops map persisted topology state into these views before workflow use.

use crate::{cdk::types::Principal, ids::CanisterRole};

///
/// IndexEntryView
///
/// Internal read-only projection of one app or subnet index entry.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexEntryView {
    pub role: CanisterRole,
    pub pid: Principal,
}

///
/// RegisteredCanisterView
///
/// Internal read-only identity and creation metadata for one registered canister.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredCanisterView {
    pub pid: Principal,
    pub created_at: u64,
}
