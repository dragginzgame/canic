pub mod ic;
pub mod icrc;
pub mod perf;
pub mod rpc;

use crate::ThisError;

///
/// InfraError
///

#[derive(Debug, ThisError)]
pub enum InfraError {
    #[error(transparent)]
    IcInfraError(#[from] ic::IcInfraError),
}
