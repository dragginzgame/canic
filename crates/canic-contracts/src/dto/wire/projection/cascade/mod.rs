//! Passive cascade transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::cascade::{StateCascadeReport, StateSnapshotInput, TopologySnapshotInput};
use candid::CandidType;
use serde::Deserialize;

/// Bounded StoreCommandFragment wire projection for cascade.

#[derive(CandidType)]
pub enum StoreCommandFragment<'a> {
    SynchronizeState(&'a StateSnapshotInput),
    SynchronizeTopology(&'a TopologySnapshotInput),
}

/// Bounded StoreCommandResponseFragment wire projection for cascade.
#[derive(CandidType, Deserialize)]
pub enum StoreCommandResponseFragment {
    SynchronizeState(StateCascadeReport),
    SynchronizeTopology,
}

/// Bounded ComponentCommandFragment wire projection for cascade.
#[derive(CandidType)]
pub enum ComponentCommandFragment<'a> {
    SynchronizeState(&'a StateSnapshotInput),
}

/// Bounded ComponentCommandResponseFragment wire projection for cascade.
#[derive(CandidType, Deserialize)]
pub enum ComponentCommandResponseFragment {
    SynchronizeState(StateCascadeReport),
}
