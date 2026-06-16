//! Module: cdk::spec::standards::icrc
//!
//! Responsibility: expose canonical ICRC Candid bindings with consistent naming.
//! Does not own: ledger behavior, consent policy, or token accounting.
//! Boundary: groups ICRC standard modules for Canic callers.

pub mod icrc2;
pub mod icrc21;
