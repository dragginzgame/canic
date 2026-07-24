//! Module: ops::topology
//!
//! Responsibility: group topology Directory and policy-input mappers.
//! Does not own: topology policy, registry storage, or endpoint DTO schemas.
//! Boundary: ops conversion layer between topology records and workflow views.

pub mod directory;
pub mod input;
