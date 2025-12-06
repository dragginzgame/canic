//!
//! WASM utilities such as hashing embedded modules with SHA-256.
//!

use sha2::{Digest, Sha256};

///
/// Compute the SHA-256 hash of a WASM byte slice.
///
#[must_use]
pub fn get_wasm_hash(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);

    hasher.finalize().to_vec()
}
