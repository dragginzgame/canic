pub mod state;
pub mod topology;

use crate::ThisError;
use candid::Principal;

///
/// SyncError
///

#[derive(Debug, ThisError)]
pub enum SyncError {
    #[error("canister not found")]
    CanisterNotFound(Principal),
}
