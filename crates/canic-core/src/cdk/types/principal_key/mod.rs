//! Canic's persisted key encoding for external IC principals.

use crate::cdk::types::Principal;
use ic_storage::storable::{Bound, CodecError, Storable};
use std::borrow::Cow;

/// Principal key in Canic-owned ordered stores; encoding carries no authority.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PrincipalKeyRecord(Principal);

impl PrincipalKeyRecord {
    /// Recover the principal at the storage projection boundary.
    #[must_use]
    pub const fn principal(&self) -> Principal {
        self.0
    }
}

impl From<Principal> for PrincipalKeyRecord {
    fn from(value: Principal) -> Self {
        Self(value)
    }
}

impl Storable for PrincipalKeyRecord {
    const BOUND: Bound = Bound::Bounded {
        max_size: 29,
        is_fixed_size: false,
    };
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, CodecError> {
        Ok(Cow::Borrowed(self.0.as_slice()))
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, CodecError> {
        Principal::try_from_slice(&bytes)
            .map(Self)
            .map_err(|_| CodecError::Invalid("principal key"))
    }
}
