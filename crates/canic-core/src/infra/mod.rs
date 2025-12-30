pub mod ic;

use crate::ThisError;

///
/// InfraError
///

#[derive(Debug, ThisError)]
pub enum InfraError {
    #[error(transparent)]
    IcInfraError(#[from] ic::IcInfraError),
}
