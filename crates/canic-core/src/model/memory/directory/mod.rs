pub mod app;
pub mod subnet;

pub(crate) use app::AppDirectory;
pub(crate) use subnet::SubnetDirectory;

use crate::{ids::CanisterRole, impl_storable_unbounded, types::Principal};
use candid::CandidType;
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
