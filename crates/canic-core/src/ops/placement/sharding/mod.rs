//! Module: ops::placement::sharding
//!
//! Responsibility: group sharding placement mappers.
//! Does not own: sharding policy, registry mutation, or endpoint DTOs.
//! Boundary: ops conversion layer for sharding placement views.

pub mod mapper;
