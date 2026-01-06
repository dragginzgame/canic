use crate::{PublicError, dto::validation::ValidationReport, workflow};

///
/// RootBootstrapApi
///

pub struct RootBootstrapApi;

impl RootBootstrapApi {
    pub async fn create_canisters() -> Result<(), PublicError> {
        workflow::bootstrap::root::root_create_canisters()
            .await
            .map_err(PublicError::from)
    }

    pub async fn import_pool_from_config() -> Result<(), PublicError> {
        workflow::bootstrap::root::root_import_pool_from_config().await;

        Ok(())
    }

    pub fn rebuild_directories_from_registry() -> Result<(), PublicError> {
        workflow::bootstrap::root::root_rebuild_directories_from_registry()
            .map_err(PublicError::from)
    }

    #[must_use]
    pub fn validate_state() -> ValidationReport {
        workflow::bootstrap::root::root_validate_state()
    }
}
