//! Module: view::state_cascade
//!
//! Responsibility: preserve the owned command contract for a selected cascade target.
//! Does not own: inventory discovery, authorization or transport.
//! Boundary: workflows derive targets from current Root or direct-child authority.

use crate::cdk::types::Principal;

/// One exact recipient and its validated inventory membership.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateCascadeTarget {
    pub canister_id: Principal,
}
