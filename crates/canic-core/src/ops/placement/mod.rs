//! Module: ops::placement
//!
//! Responsibility: group placement policy input and response mappers.
//! Does not own: placement decisions, storage mutation, or endpoint DTO schemas.
//! Boundary: ops conversion layer between storage records and placement views.

pub mod scaling;
#[cfg(feature = "sharding")]
pub mod sharding;
