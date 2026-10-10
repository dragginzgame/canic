//! Module: ids::subnet
//!
//! Responsibility: identify one exact physical IC Subnet.
//! Does not own: placement selection, trusted-network metadata, or Subnet eligibility.
//! Boundary: preserves the principal wire shape while preventing Canister/Subnet confusion.

use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::fmt;

///
/// SubnetId
///
/// Strongly typed physical IC Subnet principal used by protected Fleet bindings.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct SubnetId(Principal);

impl SubnetId {
    #[must_use]
    pub const fn from_principal(principal: Principal) -> Self {
        Self(principal)
    }

    #[must_use]
    pub const fn as_principal(&self) -> &Principal {
        &self.0
    }

    #[must_use]
    pub const fn into_principal(self) -> Principal {
        self.0
    }
}

impl CandidType for SubnetId {
    fn _ty() -> candid::types::Type {
        Principal::_ty()
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: candid::types::Serializer,
    {
        self.0.idl_serialize(serializer)
    }
}

impl fmt::Display for SubnetId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl From<Principal> for SubnetId {
    fn from(principal: Principal) -> Self {
        Self::from_principal(principal)
    }
}

impl From<SubnetId> for Principal {
    fn from(subnet: SubnetId) -> Self {
        subnet.into_principal()
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subnet_id_preserves_the_principal_candid_shape() {
        let principal = Principal::from_slice(&[7; 29]);
        let subnet = SubnetId::from_principal(principal);
        let bytes = candid::encode_one(subnet).expect("encode Subnet ID");
        let decoded_principal: Principal =
            candid::decode_one(&bytes).expect("decode Subnet ID as principal");
        let decoded_subnet: SubnetId =
            candid::decode_one(&bytes).expect("decode principal as Subnet ID");

        assert_eq!(decoded_principal, principal);
        assert_eq!(decoded_subnet, subnet);
    }
}
