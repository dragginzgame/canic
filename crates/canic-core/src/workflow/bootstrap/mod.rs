//! Bootstrap workflows.
//!
//! This module contains **async orchestration logic only**.
//! It assumes the environment has already been initialized or restored
//! by lifecycle adapters.
//!
//! It must NOT:
//! - handle IC lifecycle hooks directly
//! - depend on init payload presence
//! - perform environment seeding or restoration
//! - import directory snapshots

mod nonroot;
mod root;

pub use nonroot::*;
pub use root::*;
