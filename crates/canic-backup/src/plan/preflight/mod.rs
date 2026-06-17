//! Module: plan::preflight
//!
//! Responsibility: build and validate execution preflight contracts.
//! Does not own: authority probing, topology observation, or mutation execution.
//! Boundary: upgrades validated backup plans with accepted preflight receipts.

mod authority;
mod receipts;
mod requests;
