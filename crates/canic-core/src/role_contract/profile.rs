//! Module: role_contract::profile
//!
//! Responsibility: derive and parse the immutable compiled protocol-profile identity.
//! Does not own: Candid extraction, artifact persistence, binding selection, or endpoint dispatch.
//! Boundary: hashes exact build-owned release, role, capability, and generated-Candid evidence.

use super::RoleCapabilityKey;
use crate::ids::CanisterRole;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::str::FromStr;
use std::{collections::BTreeSet, fmt};
use thiserror::Error as ThisError;

/// Build environment carrying one canonical lowercase protocol-profile digest.
pub const PROTOCOL_PROFILE_DIGEST_ENV: &str = "CANIC_PROTOCOL_PROFILE_DIGEST";

const PROTOCOL_PROFILE_DOMAIN: &[u8] = b"canic.protocol-profile.v1";

/// SHA-256 identity of one exact release, role, capability set, and generated Candid.
#[derive(
    CandidType, Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(transparent)]
pub struct ProtocolProfileDigest([u8; 32]);

impl ProtocolProfileDigest {
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

    pub fn from_hex(value: &str) -> Result<Self, ProtocolProfileDigestParseError> {
        if value.len() != 64 {
            return Err(ProtocolProfileDigestParseError::Length(value.len()));
        }

        let mut bytes = [0_u8; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            let offset = index * 2;
            let high = decode_nibble(value.as_bytes()[offset], offset)?;
            let low = decode_nibble(value.as_bytes()[offset + 1], offset + 1)?;
            *byte = (high << 4) | low;
        }
        Ok(Self(bytes))
    }
}

impl fmt::Display for ProtocolProfileDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl FromStr for ProtocolProfileDigest {
    type Err = ProtocolProfileDigestParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::from_hex(value)
    }
}

/// Pair of generated-Candid and complete protocol-profile hashes from one authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolProfileHashes {
    pub candid_sha256: [u8; 32],
    pub protocol_profile_digest: ProtocolProfileDigest,
}

/// Derive the accepted `canic.protocol-profile.v1` identity encoding.
#[must_use]
pub fn derive_protocol_profile_hashes(
    release_identity: &str,
    role: &CanisterRole,
    capabilities: &BTreeSet<RoleCapabilityKey>,
    canonical_candid: &[u8],
) -> ProtocolProfileHashes {
    let candid_sha256 = sha256_array(canonical_candid);
    let mut hasher = Sha256::new();
    hasher.update(PROTOCOL_PROFILE_DOMAIN);
    update_length_prefixed(&mut hasher, release_identity.as_bytes());
    update_length_prefixed(&mut hasher, role.as_str().as_bytes());
    hasher.update(
        u32::try_from(capabilities.len())
            .expect("compiled capability count must fit u32")
            .to_be_bytes(),
    );
    for capability in capabilities {
        update_length_prefixed(&mut hasher, capability.manifest_name().as_bytes());
    }
    hasher.update(candid_sha256);

    ProtocolProfileHashes {
        candid_sha256,
        protocol_profile_digest: ProtocolProfileDigest::from_bytes(hasher.finalize().into()),
    }
}

fn update_length_prefixed(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update(
        u32::try_from(bytes.len())
            .expect("protocol-profile field length must fit u32")
            .to_be_bytes(),
    );
    hasher.update(bytes);
}

fn sha256_array(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn decode_nibble(byte: u8, index: usize) -> Result<u8, ProtocolProfileDigestParseError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(ProtocolProfileDigestParseError::Digit {
            index,
            byte: char::from(byte),
        }),
    }
}

/// Invalid externally supplied protocol-profile digest text.
#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum ProtocolProfileDigestParseError {
    #[error("protocol-profile digest must contain 64 lowercase hexadecimal bytes, got {0}")]
    Length(usize),
    #[error("invalid lowercase hexadecimal digit {byte:?} at index {index}")]
    Digit { index: usize, byte: char },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_encoding_is_ordered_stable_and_binds_every_input() {
        let capabilities = BTreeSet::from([
            RoleCapabilityKey::Runtime,
            RoleCapabilityKey::AutomaticTopup,
        ]);
        let first = derive_protocol_profile_hashes(
            "0.103.0",
            &CanisterRole::new("app"),
            &capabilities,
            b"service : {}\n",
        );
        let reordered = BTreeSet::from([
            RoleCapabilityKey::AutomaticTopup,
            RoleCapabilityKey::Runtime,
        ]);
        assert_eq!(
            first,
            derive_protocol_profile_hashes(
                "0.103.0",
                &CanisterRole::new("app"),
                &reordered,
                b"service : {}\n",
            )
        );
        assert_ne!(
            first,
            derive_protocol_profile_hashes(
                "0.103.1",
                &CanisterRole::new("app"),
                &capabilities,
                b"service : {}\n",
            )
        );
        assert_eq!(
            ProtocolProfileDigest::from_hex(&first.protocol_profile_digest.to_string())
                .expect("parse rendered digest"),
            first.protocol_profile_digest
        );
    }

    #[test]
    fn capability_manifest_names_are_lexicographically_ordered() {
        let names = [
            RoleCapabilityKey::AutomaticTopup,
            RoleCapabilityKey::DelegatedTokenIssuer,
            RoleCapabilityKey::DelegatedTokenVerifier,
            RoleCapabilityKey::FleetCoordinator,
            RoleCapabilityKey::Icrc21,
            RoleCapabilityKey::Index,
            RoleCapabilityKey::LocalApplicationAuthorization,
            RoleCapabilityKey::RoleAttestationSigner,
            RoleCapabilityKey::RoleAttestationVerifier,
            RoleCapabilityKey::Root,
            RoleCapabilityKey::RootControlPlane,
            RoleCapabilityKey::Runtime,
            RoleCapabilityKey::Scaling,
            RoleCapabilityKey::Sharding,
            RoleCapabilityKey::WasmStore,
        ]
        .map(RoleCapabilityKey::manifest_name);
        assert!(names.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
