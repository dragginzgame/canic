//! Module: workflow::topology::registry
//!
//! Responsibility: group read-only app and subnet registry workflow queries.
//! Does not own: registry storage mutation, endpoint authorization, or DTO schemas.
//! Boundary: workflow query namespace over registry storage ops.

pub mod query;
