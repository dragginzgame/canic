//! Module: hash
//!
//! Responsibility: provide shared backup hash formatting helpers.
//! Does not own: artifact traversal, topology canonicalization, or validation.
//! Boundary: converts bytes and digests into lowercase SHA-256 hex strings.

use sha2::{Digest, Sha256};

/// Compute the lowercase SHA-256 hex digest for one byte slice.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex_bytes(Sha256::digest(bytes))
}

/// Encode bytes as lowercase hexadecimal without allocation beyond output.
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

fn hex_char(nibble: u8) -> char {
    char::from(b"0123456789abcdef"[usize::from(nibble & 0x0f)])
}
