//! Module: workflow::topology::children
//!
//! Responsibility: expose child topology views derived for the current canister.
//! Does not own: authoritative registry state, endpoint authorization, or DTO schemas.
//! Boundary: workflow query namespace over child storage projections.

pub mod query;
