use crate::{PublicError, ops::config::ConfigOps};

///
/// export_toml
/// Endpoint-facing wrapper for exporting config as TOML.
///

pub fn export_toml() -> Result<String, PublicError> {
    ConfigOps::export_toml().map_err(PublicError::from)
}
