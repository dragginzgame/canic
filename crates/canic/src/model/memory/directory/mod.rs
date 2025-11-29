pub mod app;
pub mod subnet;

pub use app::AppDirectory;
pub use subnet::SubnetDirectory;

use crate::{
    Error, ThisError, impl_storable_unbounded,
    model::{ModelError, memory::MemoryError},
    types::{CanisterType, Principal},
};
use candid::CandidType;
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};

///
/// DirectoryError
///

#[derive(Debug, ThisError)]
pub enum DirectoryError {
    #[error("canister type not in directory: {0}")]
    TypeNotFound(CanisterType),
}

impl From<DirectoryError> for Error {
    fn from(err: DirectoryError) -> Self {
        ModelError::MemoryError(MemoryError::from(err)).into()
    }
}

///
/// PrincipalList
///

#[derive(
    CandidType, Debug, Eq, PartialEq, Deref, Default, DerefMut, Serialize, Deserialize, Clone,
)]
#[repr(transparent)]
pub struct PrincipalList(pub Vec<Principal>);

impl From<Vec<Principal>> for PrincipalList {
    fn from(vec: Vec<Principal>) -> Self {
        Self(vec)
    }
}

impl_storable_unbounded!(PrincipalList);
