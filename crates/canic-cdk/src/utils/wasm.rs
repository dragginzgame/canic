//!
//! WASM utilities such as hashing embedded modules with SHA-256.
//!

use crate::utils::hash;

///
/// Compute the SHA-256 hash of a WASM byte slice.
///
#[must_use]
pub fn get_wasm_hash(bytes: &[u8]) -> Vec<u8> {
    hash::wasm_hash(bytes)
}
