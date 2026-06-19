//! Module: view
//!
//! Responsibility: group internal read-only projections over stored or runtime state.
//! Does not own: endpoint DTOs, stable records, or workflow decisions.
//! Boundary: ops and workflow use views internally before endpoint DTO shaping.

pub mod env;
pub mod icp_refill;
pub mod placement;
pub mod topology;
