pub mod registry;

use crate::domain::value::Principal;
use thiserror::Error as ThisError;

///
/// TopologyPolicyError
///

#[derive(Debug, ThisError)]
pub enum TopologyPolicyError {
    #[error("parent {0} not found in registry")]
    ParentNotFound(Principal),

    #[error(transparent)]
    RegistryPolicy(#[from] registry::RegistryPolicyError),
}
