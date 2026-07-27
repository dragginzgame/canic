//! Module: ids::component
//!
//! Responsibility: identify declared Component topology and concrete Fleet-scoped Components.
//! Does not own: Component placement, Registry authority, lifecycle, or ID allocation.
//! Boundary: declared IDs are bounded canonical names; generated IDs use lowercase hex.

use crate::{ids::FleetKey, impl_storable_bounded};
use candid::{CandidType, Principal};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use sha2::{Digest, Sha256};
use std::{borrow::Borrow, fmt, str::FromStr};
use thiserror::Error as ThisError;

const COMPONENT_NAME_MAX_BYTES: usize = 40;
const COMPONENT_INSTANCE_ID_DOMAIN: &[u8] = b"canic:component-instance-id:v1\0";

///
/// ComponentSpecId
///
/// App-scoped identity of one declared permitted Component topology.
///

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ComponentSpecId(String);

impl ComponentSpecId {
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for ComponentSpecId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for ComponentSpecId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for ComponentSpecId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl CandidType for ComponentSpecId {
    fn _ty() -> candid::types::Type {
        candid::types::TypeInner::Text.into()
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: candid::types::Serializer,
    {
        serializer.serialize_text(self.as_str())
    }
}

impl FromStr for ComponentSpecId {
    type Err = ComponentSpecIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        validate_component_name(value).map_err(ComponentSpecIdParseError::from)?;
        Ok(Self(value.to_string()))
    }
}

impl TryFrom<String> for ComponentSpecId {
    type Error = ComponentSpecIdParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_component_name(&value).map_err(ComponentSpecIdParseError::from)?;
        Ok(Self(value))
    }
}

impl<'de> Deserialize<'de> for ComponentSpecId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_from(value).map_err(de::Error::custom)
    }
}

impl_storable_bounded!(ComponentSpecId, 64, false);

///
/// ComponentInstanceId
///
/// Generated durable Fleet-scoped identity of one concrete Component.
///

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ComponentInstanceId([u8; 32]);

impl ComponentInstanceId {
    /// Construct an ID from bytes produced by the owning root's allocator.
    ///
    /// The bytes do not become authoritative until the owning Fleet Subnet
    /// Root durably records the allocation before Component creation.
    #[must_use]
    pub const fn from_generated_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Derive one root-local allocation identity from immutable Fleet authority.
    ///
    /// The owning root must durably reserve `allocation_sequence` before any
    /// paid Component creation effect. Allocation sequences start at one and
    /// are never reused.
    #[must_use]
    pub fn from_root_allocation(
        fleet: FleetKey,
        authority_epoch: u64,
        fleet_subnet_root: Principal,
        allocation_sequence: u64,
    ) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(COMPONENT_INSTANCE_ID_DOMAIN);
        hasher.update(fleet.canonical_network_id.as_bytes());
        hasher.update(fleet.fleet_id.as_bytes());
        hasher.update(authority_epoch.to_be_bytes());
        hasher.update((fleet_subnet_root.as_slice().len() as u64).to_be_bytes());
        hasher.update(fleet_subnet_root.as_slice());
        hasher.update(allocation_sequence.to_be_bytes());
        Self(hasher.finalize().into())
    }
}

impl fmt::Display for ComponentInstanceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl CandidType for ComponentInstanceId {
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

impl FromStr for ComponentInstanceId {
    type Err = ComponentInstanceIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 64 {
            return Err(ComponentInstanceIdParseError::Length(value.len()));
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(ComponentInstanceIdParseError::CanonicalHex);
        }

        let mut bytes = [0; 32];
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            bytes[index] = (decode_nibble(pair[0]) << 4) | decode_nibble(pair[1]);
        }
        Ok(Self(bytes))
    }
}

impl Serialize for ComponentInstanceId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ComponentInstanceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(de::Error::custom)
    }
}

impl_storable_bounded!(ComponentInstanceId, 64, false);

///
/// ComponentSpecIdParseError
///
/// Typed rejection for an invalid Component Spec identifier.
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum ComponentSpecIdParseError {
    #[error("Component Spec ID must not be empty")]
    Empty,

    #[error("Component Spec ID must not exceed {max_bytes} bytes, got {actual_bytes}")]
    TooLong {
        max_bytes: usize,
        actual_bytes: usize,
    },

    #[error("Component Spec ID must use only ASCII letters, numbers, '-' or '_'")]
    InvalidCharacters,
}

///
/// ComponentInstanceIdParseError
///
/// Typed rejection for a non-canonical concrete Component identity.
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum ComponentInstanceIdParseError {
    #[error("Component Instance ID must contain exactly 64 characters, got {0}")]
    Length(usize),

    #[error("Component Instance ID must contain only lowercase hexadecimal characters")]
    CanonicalHex,
}

#[derive(Clone, Copy)]
enum ComponentNameIssue {
    Empty,
    TooLong {
        max_bytes: usize,
        actual_bytes: usize,
    },
    InvalidCharacters,
}

impl From<ComponentNameIssue> for ComponentSpecIdParseError {
    fn from(issue: ComponentNameIssue) -> Self {
        match issue {
            ComponentNameIssue::Empty => Self::Empty,
            ComponentNameIssue::TooLong {
                max_bytes,
                actual_bytes,
            } => Self::TooLong {
                max_bytes,
                actual_bytes,
            },
            ComponentNameIssue::InvalidCharacters => Self::InvalidCharacters,
        }
    }
}

fn validate_component_name(value: &str) -> Result<(), ComponentNameIssue> {
    if value.is_empty() {
        return Err(ComponentNameIssue::Empty);
    }
    if value.len() > COMPONENT_NAME_MAX_BYTES {
        return Err(ComponentNameIssue::TooLong {
            max_bytes: COMPONENT_NAME_MAX_BYTES,
            actual_bytes: value.len(),
        });
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ComponentNameIssue::InvalidCharacters);
    }
    Ok(())
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
    use crate::cdk::structures::storable::Storable;

    use super::*;

    #[test]
    fn component_spec_ids_are_bounded_canonical_names() {
        let spec = "users"
            .parse::<ComponentSpecId>()
            .expect("Component Spec ID");

        assert_eq!(spec.as_str(), "users");
        std::assert_matches!(
            "".parse::<ComponentSpecId>(),
            Err(ComponentSpecIdParseError::Empty)
        );
        std::assert_matches!(
            "bad/name".parse::<ComponentSpecId>(),
            Err(ComponentSpecIdParseError::InvalidCharacters)
        );
        std::assert_matches!(
            "a".repeat(COMPONENT_NAME_MAX_BYTES + 1)
                .parse::<ComponentSpecId>(),
            Err(ComponentSpecIdParseError::TooLong { .. })
        );
    }

    #[test]
    fn component_spec_ids_validate_serde_and_candid_input() {
        let mut invalid_bytes = Vec::new();
        ciborium::ser::into_writer("bad/name", &mut invalid_bytes)
            .expect("serialize invalid Component Spec ID");

        assert!(ciborium::de::from_reader::<ComponentSpecId, _>(invalid_bytes.as_slice()).is_err());

        let candid_bytes = candid::encode_one("projects").expect("encode Component Spec ID");
        let spec =
            candid::decode_one::<ComponentSpecId>(&candid_bytes).expect("decode Component Spec ID");
        let invalid_candid =
            candid::encode_one("bad/name").expect("encode invalid Component Spec ID");

        assert_eq!(spec.as_str(), "projects");
        assert!(candid::decode_one::<ComponentSpecId>(&invalid_candid).is_err());
    }

    #[test]
    fn component_spec_ids_fit_their_stable_bound() {
        let spec = "a"
            .repeat(COMPONENT_NAME_MAX_BYTES)
            .parse::<ComponentSpecId>()
            .expect("maximum Component Spec ID");
        let spec_bytes = spec.to_bytes();

        assert!(spec_bytes.len() <= 64);
        assert_eq!(ComponentSpecId::from_bytes(spec_bytes), spec);
    }

    #[test]
    fn component_instance_id_uses_exact_canonical_text() {
        let component = ComponentInstanceId::from_generated_bytes([0xab; 32]);
        let text = "ab".repeat(32);

        assert_eq!(component.to_string(), text);
        assert_eq!(text.parse::<ComponentInstanceId>(), Ok(component));

        let mut serde_bytes = Vec::new();
        ciborium::ser::into_writer(&component, &mut serde_bytes)
            .expect("serialize Component Instance ID");
        let decoded: ComponentInstanceId = ciborium::de::from_reader(serde_bytes.as_slice())
            .expect("decode Component Instance ID");
        assert_eq!(decoded, component);

        let candid_bytes = candid::encode_one(component).expect("encode Component Instance ID");
        let decoded: ComponentInstanceId =
            candid::decode_one(&candid_bytes).expect("decode Component Instance ID");
        let invalid_candid =
            candid::encode_one("A000000000000000000000000000000000000000000000000000000000000000")
                .expect("encode invalid Component Instance ID");

        assert_eq!(decoded, component);
        assert!(candid::decode_one::<ComponentInstanceId>(&invalid_candid).is_err());
    }

    #[test]
    fn component_instance_ids_are_domain_separated_root_allocations() {
        use crate::ids::{CanonicalNetworkId, FleetId};

        let fleet = FleetKey {
            canonical_network_id: CanonicalNetworkId::public_ic(),
            fleet_id: FleetId::from_generated_bytes([1; 32]),
        };
        let root = candid::Principal::from_slice(&[2; 29]);
        let first = ComponentInstanceId::from_root_allocation(fleet, 1, root, 1);

        assert_eq!(
            first,
            ComponentInstanceId::from_root_allocation(fleet, 1, root, 1)
        );
        assert_ne!(
            first,
            ComponentInstanceId::from_root_allocation(fleet, 1, root, 2)
        );
        assert_ne!(
            first,
            ComponentInstanceId::from_root_allocation(
                fleet,
                1,
                candid::Principal::from_slice(&[3; 29]),
                1,
            )
        );
        assert_ne!(
            first,
            ComponentInstanceId::from_root_allocation(fleet, 2, root, 1)
        );
    }

    #[test]
    fn component_instance_id_rejects_noncanonical_text() {
        std::assert_matches!(
            "ab".parse::<ComponentInstanceId>(),
            Err(ComponentInstanceIdParseError::Length(2))
        );
        std::assert_matches!(
            "A000000000000000000000000000000000000000000000000000000000000000"
                .parse::<ComponentInstanceId>(),
            Err(ComponentInstanceIdParseError::CanonicalHex)
        );
    }
}
