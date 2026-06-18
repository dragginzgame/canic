//! Module: workflow::topology
//!
//! Responsibility: group topology workflows for children, indexes, registry, and guards.
//! Does not own: stable topology records, endpoint authorization, or DTO schemas.
//! Boundary: workflow layer over topology storage ops and mutation guards.

pub mod children;
pub mod guard;
pub mod index;
pub mod registry;
