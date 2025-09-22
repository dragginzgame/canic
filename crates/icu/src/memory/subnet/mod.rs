mod children;
mod directory;
mod parents;
mod registry;

pub use children::*;
pub use directory::*;
pub use parents::*;
pub use registry::*;

use crate::{ThisError, types::CanisterType};
use candid::Principal;

///
/// SubnetError
///

#[derive(Debug, ThisError)]
pub enum SubnetError {
    #[error("canister not found: {0}")]
    PrincipalNotFound(Principal),

    #[error("canister not found: {0}")]
    TypeNotFound(CanisterType),

    #[error(transparent)]
    SubnetRegistryError(#[from] SubnetRegistryError),
}
