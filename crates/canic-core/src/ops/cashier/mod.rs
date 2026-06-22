//! Module: ops::cashier
//!
//! Responsibility: wrap Toko-approved Cashier calls and response conversions.
//! Does not own: billing workflow orchestration, endpoint authorization, or stable state.
//! Boundary: callers supply validated Cashier principals and funding decisions.

pub mod client;
pub mod conversion;
