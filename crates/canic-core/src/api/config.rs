use crate::{PublicError, workflow::config::ConfigWorkflow};

///
/// ConfigApi
///

pub struct ConfigApi;

impl ConfigApi {
    pub fn export_toml() -> Result<String, PublicError> {
        ConfigWorkflow::export_toml().map_err(PublicError::from)
    }
}
