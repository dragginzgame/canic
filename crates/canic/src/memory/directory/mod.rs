pub mod app;
pub mod subnet;

pub use app::AppDirectory;
pub use subnet::SubnetDirectory;

use crate::{
    Error, ThisError, impl_storable_unbounded,
    memory::MemoryError,
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
        MemoryError::from(err).into()
    }
}

///
/// DirectoryView
///

pub type DirectoryView = Vec<(CanisterType, PrincipalList)>;

///
/// PrincipalList
///

#[derive(
    CandidType, Debug, Eq, PartialEq, Deref, Default, DerefMut, Serialize, Deserialize, Clone,
)]
#[repr(transparent)]
pub struct PrincipalList(pub Vec<Principal>);

impl_storable_unbounded!(PrincipalList);
