//! Workspace-only internal test support for Canic self-tests.
//!
//! This crate exists to keep Canic-specific PocketIC fixtures and root
//! baseline setup out of the reusable `ic-testkit` surface.

pub mod canister;
pub mod pic;
