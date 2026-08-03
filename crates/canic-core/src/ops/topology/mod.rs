//! Module: ops::topology
//!
//! Responsibility: group topology Directory resolvers and builders.
//! Does not own: topology policy, registry storage, or endpoint DTO schemas.
//! Boundary: ops conversion layer between Directory records and workflow views.

pub mod directory;
