use crate::{PublicError, ops::config::ConfigOps};

///
/// Config Api
///

pub fn export_toml() -> Result<String, PublicError> {
    ConfigOps::export_toml().map_err(PublicError::from)
}
