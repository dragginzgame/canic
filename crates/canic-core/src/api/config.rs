use crate::{PublicError, ops::config::ConfigOps};

///
/// ConfigApi
///

pub struct ConfigApi;

impl ConfigApi {
    pub fn export_toml() -> Result<String, PublicError> {
        ConfigOps::export_toml().map_err(PublicError::from)
    }
}
