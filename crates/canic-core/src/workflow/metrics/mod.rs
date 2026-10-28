//! Module: workflow::metrics
//!
//! Responsibility: group read-only metrics workflow queries.
//! Does not own: metric arithmetic, storage codecs, endpoint authorization or DTO schemas.
//! Boundary: explicit recording/lifecycle orchestration and runtime projections.

pub mod providers;
pub mod publication;
pub mod query;
