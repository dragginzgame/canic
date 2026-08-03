//! Module: api::state
//!
//! Responsibility: expose read-only Fleet state to configured canisters.
//! Does not own: root mutation fanout or control-plane inventory selection.
//! Boundary: root mutation is exposed by the root control-plane facade.

/// Re-export of read-only state query surfaces.
pub use crate::workflow::state::query::FleetStateQuery;
