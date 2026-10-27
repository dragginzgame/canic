//! Module: ids::release_set
//!
//! Responsibility: identify one exact canonical Fleet Subnet Root release-set manifest.
//! Does not own: manifest encoding, artifact storage, publication, or active-set policy.
//! Boundary: the host derives this digest from validated canonical manifest bytes.

use crate::ids::ReleaseBuildId;
use serde::{Deserialize, Serialize};
use std::{fmt, fmt::Display};

///
/// ReleaseSetDigest
///
/// SHA-256 identity of one canonical Fleet Subnet Root release-set manifest.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ReleaseSetDigest([u8; 32]);

impl ReleaseSetDigest {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl Display for ReleaseSetDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

///
/// FleetSubnetRootReleaseSet
///
/// Exact release-build and manifest identity admitted for one Fleet Subnet Root.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetSubnetRootReleaseSet {
    pub release_build_id: ReleaseBuildId,
    pub manifest_digest: ReleaseSetDigest,
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ReleaseBuildNonce;

    #[test]
    fn digest_has_exact_bytes_text_and_wire_representation() {
        let digest = ReleaseSetDigest::from_bytes([0xab; 32]);

        assert_eq!(digest.as_bytes(), &[0xab; 32]);
        assert_eq!(digest.into_bytes(), [0xab; 32]);
        assert_eq!(digest.to_string(), "ab".repeat(32));

        let bytes = crate::protocol::authority::encode(digest).expect("encode Release Set digest");
        assert_eq!(
            crate::protocol::authority::decode::<ReleaseSetDigest>(&bytes)
                .expect("decode Release Set digest"),
            digest
        );

        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [7; 32],
            )),
            manifest_digest: digest,
        };
        let bytes = crate::protocol::authority::encode(release_set)
            .expect("encode root Release Set identity");
        assert_eq!(
            crate::protocol::authority::decode::<FleetSubnetRootReleaseSet>(&bytes)
                .expect("decode root Release Set identity"),
            release_set
        );
    }
}
