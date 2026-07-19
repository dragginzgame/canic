//! Module: plan
//!
//! Responsibility: build and validate backup plans and preflight contracts.
//! Does not own: registry discovery, journal execution, or artifact storage.
//! Boundary: converts selected topology into executable backup operations.

mod build;
mod error;
mod preflight;
#[cfg(test)]
mod tests;
mod types;
mod validation;

use build::build_backup_phases;
pub use build::{BackupPlanBuildInput, build_backup_plan, resolve_backup_selector};
pub use error::BackupPlanError;
pub use types::*;
