//! Module: plan::preflight
//!
//! Responsibility: build and validate execution preflight contracts.
//! Does not own: authority probing, topology observation, or mutation execution.
//! Boundary: describes and validates retained backup preflight evidence.

mod receipts;
mod requests;
