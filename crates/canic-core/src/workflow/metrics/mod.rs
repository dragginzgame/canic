//! Module: workflow::metrics
//!
//! Responsibility: group read-only metrics workflow queries.
//! Does not own: metric recording, endpoint authorization, or DTO schemas.
//! Boundary: workflow query namespace over runtime metrics projections.

pub mod query;
