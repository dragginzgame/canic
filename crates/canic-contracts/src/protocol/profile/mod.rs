//! Immutable compiled protocol-profile identity, independent of runtime policy.

use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error as ThisError;

#[derive(
    CandidType, Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(transparent)]
pub struct ProtocolProfileDigest([u8; 32]);

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum ProtocolProfileDigestParseError {
    #[error("protocol-profile digest must contain 64 lowercase hexadecimal bytes, got {0}")]
    Length(usize),
    #[error("invalid lowercase hexadecimal digit {byte:?} at index {index}")]
    Digit { index: usize, byte: char },
}

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
