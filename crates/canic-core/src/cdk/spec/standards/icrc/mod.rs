//! Module: cdk::spec::standards::icrc
//!
//! Responsibility: expose internal canonical ICRC ledger Candid bindings.
//! Does not own: ledger behavior, public protocol DTOs, or token accounting.
//! Boundary: keeps ICRC-2 wire types with Canic's internal ledger adapter.

pub mod icrc2;
