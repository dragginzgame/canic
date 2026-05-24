//!
//! Shared SHA-256 helpers for wasm/module identity and hex rendering.
//!

use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

///
/// HashBytes
///

pub type HashBytes = Vec<u8>;

/// Compute SHA-256 bytes from an in-memory byte slice.
#[must_use]
pub fn sha256_bytes(bytes: &[u8]) -> HashBytes {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().to_vec()
}

/// Compute lowercase hexadecimal SHA-256 from an in-memory byte slice.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex_bytes(sha256_bytes(bytes))
}

/// Compute raw wasm module hash bytes.
#[must_use]
pub fn wasm_hash(bytes: &[u8]) -> HashBytes {
    sha256_bytes(bytes)
}

/// Compute lowercase hexadecimal wasm module hash.
#[must_use]
pub fn wasm_hash_hex(bytes: &[u8]) -> String {
    sha256_hex(bytes)
}

/// Render one byte slice as lowercase hexadecimal.
#[must_use]
pub fn hex_bytes(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut encoded = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        encoded.push(hex_char(byte >> 4));
        encoded.push(hex_char(byte & 0x0f));
    }

    encoded
}

/// Decode one even-length hexadecimal string into bytes.
pub fn decode_hex(hex: &str) -> Result<HashBytes, DecodeHexError> {
    if !hex.len().is_multiple_of(2) {
        return Err(DecodeHexError::OddLength(hex.len()));
    }

    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for index in (0..hex.len()).step_by(2) {
        let high = decode_nibble(hex.as_bytes()[index], index)?;
        let low = decode_nibble(hex.as_bytes()[index + 1], index + 1)?;
        bytes.push((high << 4) | low);
    }

    Ok(bytes)
}

// Convert one four-bit nibble to lowercase hexadecimal.
fn hex_char(nibble: u8) -> char {
    char::from(b"0123456789abcdef"[usize::from(nibble & 0x0f)])
}

// Decode one ASCII hex digit.
fn decode_nibble(byte: u8, index: usize) -> Result<u8, DecodeHexError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(DecodeHexError::InvalidDigit {
            index,
            byte: char::from(byte),
        }),
    }
}

///
/// DecodeHexError
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodeHexError {
    OddLength(usize),

    InvalidDigit { index: usize, byte: char },
}

impl fmt::Display for DecodeHexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OddLength(length) => {
                write!(formatter, "hex string must have even length, got {length}")
            }
            Self::InvalidDigit { index, byte } => {
                write!(formatter, "invalid hex digit {byte:?} at index {index}")
            }
        }
    }
}

impl Error for DecodeHexError {}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb924\
                                27ae41e4649b934ca495991b7852b855";

    #[test]
    fn wasm_hash_hex_matches_sha256_vector() {
        assert_eq!(wasm_hash_hex(&[]), EMPTY_SHA256);
    }

    #[test]
    fn hex_round_trip_accepts_upper_and_lowercase() {
        assert_eq!(decode_hex("01aBff").expect("decode hex"), vec![1, 171, 255]);
        assert_eq!(hex_bytes([1, 171, 255]), "01abff");
    }

    #[test]
    fn decode_hex_rejects_invalid_input() {
        assert!(matches!(decode_hex("f"), Err(DecodeHexError::OddLength(1))));
        assert!(matches!(
            decode_hex("0g"),
            Err(DecodeHexError::InvalidDigit { index: 1, .. })
        ));
    }
}
