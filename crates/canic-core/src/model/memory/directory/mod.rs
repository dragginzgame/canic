pub mod app;
pub mod subnet;

pub use app::AppDirectory;
pub use subnet::SubnetDirectory;

use crate::{ids::CanisterRole, memory::impl_storable_unbounded};
use candid::{CandidType, Principal};
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};

///
/// DirectoryView
///

pub type DirectoryView = Vec<(CanisterRole, PrincipalList)>;

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
