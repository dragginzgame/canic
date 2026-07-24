//! Module: ids::network
//!
//! Responsibility: identify one IC network by its verified trust identity.
//! Does not own: trust-anchor enrollment, environment profiles, or network access.
//! Boundary: derives and serializes the canonical 32-byte network identity.

use crate::domain::auth::{
    IC_ROOT_PUBLIC_KEY_RAW_LENGTH, ic_root_public_key_raw_from_der_or_raw,
    mainnet_ic_root_public_key_der,
};
use candid::CandidType;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use sha2::{Digest, Sha256};
use std::{fmt, str::FromStr};
use thiserror::Error as ThisError;

const CANONICAL_NETWORK_ID_DOMAIN: &[u8] = b"canic:canonical-network-id\0";

///
/// CanonicalNetworkId
///
/// Trust-derived identity for one IC network.
///

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CanonicalNetworkId([u8; 32]);

impl CanonicalNetworkId {
    fn from_network_trust_identity(network_trust_identity: [u8; 32]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(CANONICAL_NETWORK_ID_DOMAIN);
        hasher.update(network_trust_identity);
        Self(hasher.finalize().into())
    }

    /// Derive the public IC identity from Canic's compiled, pinned DER trust anchor.
    #[must_use]
    pub fn public_ic() -> Self {
        Self::from_der_bytes_unchecked(&mainnet_ic_root_public_key_der())
    }

    /// Validate and derive the identity of one explicitly enrolled DER trust anchor.
    pub fn from_der_root_trust_anchor(
        root_key: &[u8],
    ) -> Result<Self, CanonicalNetworkTrustAnchorError> {
        if root_key.len() == IC_ROOT_PUBLIC_KEY_RAW_LENGTH {
            return Err(CanonicalNetworkTrustAnchorError::RawKey);
        }
        ic_root_public_key_raw_from_der_or_raw(root_key)
            .map_err(CanonicalNetworkTrustAnchorError::InvalidDer)?;
        Ok(Self::from_der_bytes_unchecked(root_key))
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    fn from_der_bytes_unchecked(root_key: &[u8]) -> Self {
        Self::from_network_trust_identity(Sha256::digest(root_key).into())
    }
}

impl fmt::Display for CanonicalNetworkId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl CandidType for CanonicalNetworkId {
    fn _ty() -> candid::types::Type {
        candid::types::TypeInner::Text.into()
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: candid::types::Serializer,
    {
        serializer.serialize_text(&self.to_string())
    }
}

impl FromStr for CanonicalNetworkId {
    type Err = CanonicalNetworkIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 64 {
            return Err(CanonicalNetworkIdParseError::Length(value.len()));
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(CanonicalNetworkIdParseError::CanonicalHex);
        }

        let mut bytes = [0; 32];
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            bytes[index] = (decode_nibble(pair[0]) << 4) | decode_nibble(pair[1]);
        }
        Ok(Self(bytes))
    }
}

impl Serialize for CanonicalNetworkId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for CanonicalNetworkId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(de::Error::custom)
    }
}

///
/// CanonicalNetworkIdParseError
///
/// Typed rejection for a non-canonical network identity.
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum CanonicalNetworkIdParseError {
    #[error("canonical network ID must contain exactly 64 characters, got {0}")]
    Length(usize),

    #[error("canonical network ID must contain only lowercase hexadecimal characters")]
    CanonicalHex,
}

///
/// CanonicalNetworkTrustAnchorError
///
/// Typed rejection for an invalid enrolled IC root trust anchor.
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum CanonicalNetworkTrustAnchorError {
    #[error("raw root public key bytes are not an enrolled DER trust anchor")]
    RawKey,

    #[error("{0}")]
    InvalidDer(String),
}

fn decode_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("canonical hex was validated before decoding"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_domain_separated_network_identity() {
        let identity = CanonicalNetworkId::from_network_trust_identity([7; 32]);

        let mut expected = Sha256::new();
        expected.update(CANONICAL_NETWORK_ID_DOMAIN);
        expected.update([7; 32]);
        assert_eq!(identity.as_bytes(), &<[u8; 32]>::from(expected.finalize()));
    }

    #[test]
    fn text_uses_exact_lowercase_hex() {
        let identity = CanonicalNetworkId::from_network_trust_identity([11; 32]);
        let text = identity.to_string();

        assert_eq!(text.len(), 64);
        assert!(
            text.bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
        assert_eq!(text.parse::<CanonicalNetworkId>(), Ok(identity));
    }

    #[test]
    fn parsing_rejects_noncanonical_text() {
        std::assert_matches!(
            "ab".parse::<CanonicalNetworkId>(),
            Err(CanonicalNetworkIdParseError::Length(2))
        );
        std::assert_matches!(
            "A000000000000000000000000000000000000000000000000000000000000000"
                .parse::<CanonicalNetworkId>(),
            Err(CanonicalNetworkIdParseError::CanonicalHex)
        );
        std::assert_matches!(
            "g000000000000000000000000000000000000000000000000000000000000000"
                .parse::<CanonicalNetworkId>(),
            Err(CanonicalNetworkIdParseError::CanonicalHex)
        );
    }

    #[test]
    fn public_and_enrolled_der_paths_share_the_canonical_der_derivation() {
        let der = mainnet_ic_root_public_key_der();

        assert_eq!(
            CanonicalNetworkId::from_der_root_trust_anchor(&der).expect("valid DER"),
            CanonicalNetworkId::public_ic()
        );
        std::assert_matches!(
            CanonicalNetworkId::from_der_root_trust_anchor(&[0; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]),
            Err(CanonicalNetworkTrustAnchorError::RawKey)
        );
    }
}
