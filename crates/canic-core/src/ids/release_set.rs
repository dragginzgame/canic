//! Module: ids::release_set
//!
//! Responsibility: identify one exact canonical Fleet Subnet Root release-set manifest.
//! Does not own: manifest encoding, artifact storage, publication, or active-set policy.
//! Boundary: the host derives this digest from validated canonical manifest bytes.

use std::fmt::{self, Display};

use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// ReleaseSetDigest
///
/// SHA-256 identity of one canonical Fleet Subnet Root release-set manifest.
///

#[derive(
    CandidType, Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(transparent)]
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_has_exact_bytes_text_and_candid_representation() {
        let digest = ReleaseSetDigest::from_bytes([0xab; 32]);

        assert_eq!(digest.as_bytes(), &[0xab; 32]);
        assert_eq!(digest.into_bytes(), [0xab; 32]);
        assert_eq!(digest.to_string(), "ab".repeat(32));

        let bytes = candid::encode_one(digest).expect("encode Release Set digest");
        assert_eq!(
            candid::decode_one::<ReleaseSetDigest>(&bytes).expect("decode Release Set digest"),
            digest
        );
    }
}
