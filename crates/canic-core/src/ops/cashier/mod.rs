//! Module: ops::cashier
//!
//! Responsibility: wrap Toko-approved Cashier calls and response conversions.
//! Does not own: billing workflow orchestration, endpoint authorization, or stable state.
//! Boundary: workflow calls these ops after billing config and policy checks.

pub mod client;
pub mod conversion;
