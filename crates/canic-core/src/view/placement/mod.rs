//! Module: view::placement
//!
//! Responsibility: group placement read-only projection types.
//! Does not own: placement policy, storage records, or endpoint DTOs.
//! Boundary: ops mappers produce these views for placement workflows.

pub mod scaling;
#[cfg(feature = "sharding")]
pub mod sharding;
