//! Module: fleet_ensure::view
//!
//! Responsibility: expose read-only funding projections for operator review.
//! Does not own: persisted authority, funding admission or cycle effects.
//! Boundary: projections describe assumptions and never authorize spending.

pub mod startup_funding;
