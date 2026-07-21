//! Module: workflow::config
//!
//! Responsibility: provide the workflow facade for config export.
//! Does not own: configuration storage, timer registration, or endpoint authorization.
//! Boundary: delegates config serialization to ops.

use crate::{InternalError, ops::config::ConfigOps};

///
/// ConfigWorkflow
///
/// Workflow facade for configuration export.
///

pub struct ConfigWorkflow;

impl ConfigWorkflow {
    pub fn export_toml() -> Result<String, InternalError> {
        ConfigOps::export_toml()
    }
}
