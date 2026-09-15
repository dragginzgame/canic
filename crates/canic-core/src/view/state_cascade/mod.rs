//! Module: view::state_cascade
//!
//! Responsibility: preserve the owned command contract for a selected cascade target.
//! Does not own: inventory discovery, authorization or transport.
//! Boundary: workflows derive targets from current Root or direct-child authority.

use crate::cdk::types::Principal;

/// Current command surface owned by a state-cascade recipient.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateCascadeEndpoint {
    Component,
    Store,
}

/// One exact recipient and its current role-owned command surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateCascadeTarget {
    pub canister_id: Principal,
    pub endpoint: StateCascadeEndpoint,
}
