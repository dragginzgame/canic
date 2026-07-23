//! Module: ids::fleet
//!
//! Responsibility: identify one installed Fleet independently of its display name.
//! Does not own: Fleet ID generation, host catalog publication, or activation state.
//! Boundary: IDs use canonical lowercase hexadecimal text; names are validated labels.

use super::{AppId, CanonicalNetworkId};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::{fmt, str::FromStr};
use thiserror::Error as ThisError;

const FLEET_NAME_MAX_BYTES: usize = 40;

///
/// FleetId
///
/// Generated durable identity for one Fleet.
///

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FleetId([u8; 32]);

impl FleetId {
    /// Construct an ID from bytes produced by the host's cryptographic generator.
    ///
    /// The bytes do not become authoritative until the activation journal
    /// durably records them.
    #[must_use]
    pub const fn from_generated_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for FleetId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl FromStr for FleetId {
    type Err = FleetIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 64 {
            return Err(FleetIdParseError::Length(value.len()));
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(FleetIdParseError::CanonicalHex);
        }

        let mut bytes = [0; 32];
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            bytes[index] = (decode_nibble(pair[0]) << 4) | decode_nibble(pair[1]);
        }
        Ok(Self(bytes))
    }
}

impl Serialize for FleetId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for FleetId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(de::Error::custom)
    }
}

///
/// FleetName
///
/// Immutable operator-facing label for one Fleet.
///

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct FleetName(String);

impl FleetName {
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for FleetName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for FleetName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for FleetName {
    type Err = FleetNameParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        validate_fleet_name(value)?;
        Ok(Self(value.to_string()))
    }
}

impl TryFrom<String> for FleetName {
    type Error = FleetNameParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_fleet_name(&value)?;
        Ok(Self(value))
    }
}

impl<'de> Deserialize<'de> for FleetName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_from(value).map_err(de::Error::custom)
    }
}

///
/// FleetKey
///
/// Complete network-qualified Fleet identity.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetKey {
    pub network: CanonicalNetworkId,
    pub fleet_id: FleetId,
}

///
/// FleetBinding
///
/// Immutable binding between one installed Fleet and its source App.
///

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetBinding {
    pub fleet: FleetKey,
    pub app: AppId,
}

///
/// FleetIdParseError
///
/// Typed rejection for a non-canonical Fleet identity.
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum FleetIdParseError {
    #[error("Fleet ID must contain exactly 64 characters, got {0}")]
    Length(usize),

    #[error("Fleet ID must contain only lowercase hexadecimal characters")]
    CanonicalHex,
}

///
/// FleetNameParseError
///
/// Typed rejection for an invalid Fleet label.
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum FleetNameParseError {
    #[error("Fleet name must not be empty")]
    Empty,

    #[error("Fleet name must not exceed {max_bytes} bytes, got {actual_bytes}")]
    TooLong {
        max_bytes: usize,
        actual_bytes: usize,
    },

    #[error("Fleet name must use only ASCII letters, numbers, '-' or '_'")]
    InvalidCharacters,
}

fn validate_fleet_name(value: &str) -> Result<(), FleetNameParseError> {
    if value.is_empty() {
        return Err(FleetNameParseError::Empty);
    }
    if value.len() > FLEET_NAME_MAX_BYTES {
        return Err(FleetNameParseError::TooLong {
            max_bytes: FLEET_NAME_MAX_BYTES,
            actual_bytes: value.len(),
        });
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(FleetNameParseError::InvalidCharacters);
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
    use super::*;

    #[test]
    fn fleet_id_uses_exact_canonical_text() {
        let fleet_id = FleetId::from_generated_bytes([0xab; 32]);
        let text = "ab".repeat(32);

        assert_eq!(fleet_id.to_string(), text);
        assert_eq!(text.parse::<FleetId>(), Ok(fleet_id));
    }

    #[test]
    fn fleet_id_rejects_noncanonical_text() {
        std::assert_matches!("ab".parse::<FleetId>(), Err(FleetIdParseError::Length(2)));
        std::assert_matches!(
            "A000000000000000000000000000000000000000000000000000000000000000".parse::<FleetId>(),
            Err(FleetIdParseError::CanonicalHex)
        );
    }

    #[test]
    fn fleet_name_accepts_only_the_existing_canonical_name_shape() {
        let name = "toko-production"
            .parse::<FleetName>()
            .expect("canonical Fleet name");

        assert_eq!(name.as_str(), "toko-production");
        std::assert_matches!("".parse::<FleetName>(), Err(FleetNameParseError::Empty));
        std::assert_matches!(
            "bad/name".parse::<FleetName>(),
            Err(FleetNameParseError::InvalidCharacters)
        );
        std::assert_matches!(
            "a".repeat(FLEET_NAME_MAX_BYTES + 1).parse::<FleetName>(),
            Err(FleetNameParseError::TooLong { .. })
        );
    }

    #[test]
    fn fleet_binding_keeps_app_and_network_separate_from_the_label() {
        let network = CanonicalNetworkId::public_ic();
        let fleet_id = FleetId::from_generated_bytes([7; 32]);
        let binding = FleetBinding {
            fleet: FleetKey { network, fleet_id },
            app: AppId::from("toko"),
        };

        assert_eq!(binding.fleet.network, network);
        assert_eq!(binding.fleet.fleet_id, fleet_id);
        assert_eq!(binding.app.as_str(), "toko");
    }
}
