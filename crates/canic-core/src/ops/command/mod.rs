pub mod request;
pub mod response;

use crate::{Error, ThisError, ops::OpsError};

///
/// CommandOpsError
///

#[derive(Debug, ThisError)]
pub enum CommandOpsError {
    #[error(transparent)]
    RequestOpsError(#[from] request::RequestOpsError),
}

impl From<CommandOpsError> for Error {
    fn from(err: CommandOpsError) -> Self {
        OpsError::from(err).into()
    }
}
