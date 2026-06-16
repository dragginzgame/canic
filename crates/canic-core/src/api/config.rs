//! Module: api::config
//!
//! Responsibility: public config export facade for endpoint callers.
//! Does not own: config storage, parsing policy, or serialization format rules.
//! Boundary: maps config workflow errors into public API errors.

use crate::{dto::error::Error, workflow::config::ConfigWorkflow};

///
/// ConfigApi
///
/// Thin endpoint-facing facade for exported Canic configuration.
///

pub struct ConfigApi;

impl ConfigApi {
    /// Export the current Canic config as TOML.
    pub fn export_toml() -> Result<String, Error> {
        ConfigWorkflow::export_toml().map_err(Error::from)
    }
}
