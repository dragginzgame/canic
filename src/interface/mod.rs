pub mod cascade;

use crate::interface::cascade::CascadeError;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// InterfaceError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum InterfaceError {
    #[error(transparent)]
    CascadeError(#[from] CascadeError),
}
