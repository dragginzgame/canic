//! Module: workflow::log
//!
//! Responsibility: group read-only runtime log workflow queries.
//! Does not own: log storage mutation, endpoint authorization, or DTO schemas.
//! Boundary: workflow query namespace over runtime log ops.

pub mod query;
