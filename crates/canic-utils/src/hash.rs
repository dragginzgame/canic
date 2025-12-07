//!
//! xxHash3 hashing utilities.
//!
//! These functions provide **fast, deterministic, non-cryptographic hashes**
//! that work reliably on the Internet Computer.
//!
//! They are used across both **canic** (runtime layer) and **mimic**
//! (framework layer) for tasks that require uniform and reproducible hashing
//! such as:
//!   - routing / sharding decisions,
//!   - cache keys or internal identifiers,
//!   - stable yet non-secure fingerprinting of data.
//!
//! ✅ Extremely fast (optimized for 64-bit architectures)
//! ✅ Deterministic across replicas and platforms
//! ⚠️ Not cryptographically secure — do not use for signatures or certified data
//!
//! Reference: <https://cyan4973.github.io/xxHash/>
//!
use xxhash_rust::xxh3::{xxh3_64, xxh3_128};

pub use xxhash_rust::xxh3::Xxh3;

/// Return a u64 hash from the provided bytes using the xxh3 hash algorithm.
#[must_use]
pub fn hash_u64(bytes: &[u8]) -> u64 {
    xxh3_64(bytes)
}

/// Return a u128 hash from the provided bytes using the xxh3 hash algorithm.
#[must_use]
pub fn hash_u128(bytes: &[u8]) -> u128 {
    xxh3_128(bytes)
}
