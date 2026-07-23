//! Module: ids::release_build
//!
//! Responsibility: identify one pre-planned release build without depending on its artifacts.
//! Does not own: nonce generation, durable build plans, manifests, or deployment admission.
//! Boundary: the host supplies random nonce bytes and every selected Wasm receives the derived ID.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use sha2::{Digest, Sha256};
use std::{fmt, str::FromStr};
use thiserror::Error as ThisError;

const RELEASE_BUILD_ID_DOMAIN: &[u8] = b"canic:release-build-id\0";
const RELEASE_BUILD_NONCE_BYTES: u64 = 32;

/// Host-to-build-script environment variable carrying one planned release identity.
pub const RELEASE_BUILD_ID_ENV: &str = "CANIC_RELEASE_BUILD_ID";

///
/// ReleaseBuildNonce
///
/// Random host-owned input durably recorded before a release build starts.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReleaseBuildNonce([u8; 32]);

impl ReleaseBuildNonce {
    /// Construct a nonce from bytes supplied by the host's cryptographic generator.
    #[must_use]
    pub const fn from_random_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

///
/// ReleaseBuildId
///
/// Non-circular identity embedded into every Wasm in one planned release build.
///

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ReleaseBuildId([u8; 32]);

impl ReleaseBuildId {
    /// Derive the only valid release-build identity from a journalled nonce.
    #[must_use]
    pub fn from_nonce(nonce: ReleaseBuildNonce) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(RELEASE_BUILD_ID_DOMAIN);
        hasher.update(RELEASE_BUILD_NONCE_BYTES.to_be_bytes());
        hasher.update(nonce.as_bytes());
        Self(hasher.finalize().into())
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for ReleaseBuildId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl FromStr for ReleaseBuildId {
    type Err = ReleaseBuildIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 64 {
            return Err(ReleaseBuildIdParseError::Length(value.len()));
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(ReleaseBuildIdParseError::CanonicalHex);
        }

        let mut bytes = [0; 32];
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            bytes[index] = (decode_nibble(pair[0]) << 4) | decode_nibble(pair[1]);
        }
        Ok(Self(bytes))
    }
}

impl Serialize for ReleaseBuildId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ReleaseBuildId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(de::Error::custom)
    }
}

///
/// ReleaseBuildIdParseError
///
/// Typed rejection for a non-canonical release-build identity.
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum ReleaseBuildIdParseError {
    #[error("release build ID must contain exactly 64 characters, got {0}")]
    Length(usize),

    #[error("release build ID must contain only lowercase hexadecimal characters")]
    CanonicalHex,
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
    fn release_build_id_uses_the_exact_non_circular_derivation() {
        let nonce = ReleaseBuildNonce::from_random_bytes([7; 32]);
        let expected: [u8; 32] = Sha256::digest(
            [
                RELEASE_BUILD_ID_DOMAIN,
                RELEASE_BUILD_NONCE_BYTES.to_be_bytes().as_slice(),
                nonce.as_bytes(),
            ]
            .concat(),
        )
        .into();

        assert_eq!(ReleaseBuildId::from_nonce(nonce).as_bytes(), &expected);
    }

    #[test]
    fn release_build_id_text_is_exact_and_canonical() {
        let id = ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([11; 32]));
        let text = id.to_string();

        assert_eq!(text.len(), 64);
        assert_eq!(text.parse::<ReleaseBuildId>(), Ok(id));
        std::assert_matches!(
            text.to_uppercase().parse::<ReleaseBuildId>(),
            Err(ReleaseBuildIdParseError::CanonicalHex)
        );
    }
}
