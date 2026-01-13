use crate::{dto::error::Error, workflow::config::ConfigWorkflow};

///
/// ConfigApi
///

pub struct ConfigApi;

impl ConfigApi {
    pub fn export_toml() -> Result<String, Error> {
        ConfigWorkflow::export_toml().map_err(Error::from)
    }
}
