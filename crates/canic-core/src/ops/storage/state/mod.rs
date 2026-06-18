//! Module: ops::storage::state
//!
//! Responsibility: expose deterministic state storage operations and mappers.
//! Does not own: stable state schemas, workflow orchestration, or endpoint DTOs.
//! Boundary: storage ops between state workflows and stable state records.

pub mod app;
pub mod mapper;
pub mod subnet;
