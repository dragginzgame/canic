//! Module: workflow::topology::index
//!
//! Responsibility: group read-only app and subnet index workflow queries.
//! Does not own: index storage mutation, endpoint authorization, or DTO schemas.
//! Boundary: workflow query namespace over index storage ops.

pub mod query;
