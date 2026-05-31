use super::{
    DirectoryWorkflow,
    state::{DirectoryWorkflowError, available_pool_names},
};
use crate::{InternalError, config::schema::DirectoryPool, ops::config::ConfigOps};

impl DirectoryWorkflow {
    // Resolve the configured pool definition for the current directory-bearing parent.
    pub(super) fn get_directory_pool_cfg(pool: &str) -> Result<DirectoryPool, InternalError> {
        let directory = ConfigOps::current_directory_config()?
            .ok_or(DirectoryWorkflowError::DirectoryDisabled)?;
        let available = available_pool_names(&directory);

        directory
            .pools
            .get(pool)
            .cloned()
            .ok_or_else(|| DirectoryWorkflowError::UnknownPool {
                requested: pool.to_string(),
                available,
            })
            .map_err(InternalError::from)
    }
}
