//!
//! Thread-local PRNG seeded externally (raw_rand recommended).
//!
//! The IC executes canister code single-threaded, so `RefCell` provides
//! sufficient interior mutability without locking or poisoning semantics.
//!
//! Use update calls for randomness so the PRNG state advances, and seed in
//! init + post_upgrade via timers.
//!
use rand_chacha::{
    ChaCha20Rng,
    rand_core::{RngCore, SeedableRng},
};
use std::cell::RefCell;

thread_local! {
    static RNG: RefCell<Option<ChaCha20Rng>> = const { RefCell::new(None) };
}

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

///
/// RngError
/// Errors raised when randomness is unavailable.
///

#[derive(Debug)]
pub enum RngError {
    RngNotInitialized(String),
}

impl RngError {
    fn not_initialized() -> Self {
        Self::RngNotInitialized("Randomness is not initialized. Please try again later".to_string())
    }
}

// -----------------------------------------------------------------------------
// Seeding
// -----------------------------------------------------------------------------

/// Seed the RNG with a 32-byte value (e.g. management canister `raw_rand` output).
pub fn seed_from(seed: [u8; 32]) {
    RNG.with_borrow_mut(|rng| {
        *rng = Some(ChaCha20Rng::from_seed(seed));
    });
}

/// Returns true if the RNG has been seeded.
#[must_use]
pub fn is_seeded() -> bool {
    RNG.with_borrow(Option::is_some)
}

fn with_rng<T>(f: impl FnOnce(&mut ChaCha20Rng) -> T) -> Result<T, RngError> {
    RNG.with_borrow_mut(|rng| match rng.as_mut() {
        Some(rand) => Ok(f(rand)),
        None => Err(RngError::not_initialized()),
    })
}

// -----------------------------------------------------------------------------
// Random bytes
// -----------------------------------------------------------------------------

/// Fill the provided buffer with random bytes.
pub fn fill_bytes(dest: &mut [u8]) -> Result<(), RngError> {
    with_rng(|rand| rand.fill_bytes(dest))
}

/// Produce random bytes using the shared RNG.
pub fn random_bytes(size: usize) -> Result<Vec<u8>, RngError> {
    let mut buf = vec![0u8; size];
    fill_bytes(&mut buf)?;
    Ok(buf)
}

/// Produce hex-encoded random bytes using the shared RNG.
pub fn random_hex(size: usize) -> Result<String, RngError> {
    let bytes = random_bytes(size)?;
    Ok(hex::encode(bytes))
}

/// Produce an 8-bit random value (derived from `next_u16`).
pub fn next_u8() -> Result<u8, RngError> {
    Ok((next_u16()? & 0xFF) as u8)
}

/// Produce a 16-bit random value from the shared RNG.
#[allow(clippy::cast_possible_truncation)]
pub fn next_u16() -> Result<u16, RngError> {
    with_rng(|rand| rand.next_u32() as u16)
}

/// Produce a 32-bit random value from the shared RNG.
pub fn next_u32() -> Result<u32, RngError> {
    with_rng(RngCore::next_u32)
}

/// Produce a 64-bit random value from the shared RNG.
pub fn next_u64() -> Result<u64, RngError> {
    with_rng(RngCore::next_u64)
}

/// Produce a 128-bit random value from the shared RNG.
pub fn next_u128() -> Result<u128, RngError> {
    with_rng(|rand| {
        let hi = u128::from(rand.next_u64());
        let lo = u128::from(rand.next_u64());
        (hi << 64) | lo
    })
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_u64s() {
        use std::collections::HashSet;

        seed_from([7; 32]);

        let mut set = HashSet::new();
        while set.len() < 1000 {
            let random_value = next_u64().expect("seeded RNG");
            assert!(set.insert(random_value), "value already in set");
        }
    }

    #[test]
    fn test_rng_reseeding() {
        seed_from([1; 32]);
        let first = next_u64().expect("seeded RNG");
        seed_from([2; 32]);
        let second = next_u64().expect("seeded RNG");

        assert_ne!(
            first, second,
            "RNGs with different seeds unexpectedly produced the same value"
        );
    }

    #[test]
    fn test_determinism_with_fixed_seed() {
        let seed = [42u8; 32];
        seed_from(seed);

        let values: Vec<u64> = (0..100).map(|_| next_u64().expect("seeded RNG")).collect();

        seed_from(seed);
        for value in values {
            assert_eq!(next_u64().expect("seeded RNG"), value);
        }
    }

    #[test]
    fn test_missing_seed_errors() {
        RNG.with_borrow_mut(|rng| {
            *rng = None;
        });

        assert!(matches!(
            random_bytes(8),
            Err(RngError::RngNotInitialized(_))
        ));
    }

    #[test]
    fn test_random_hex_length() {
        seed_from([9; 32]);

        let value = random_hex(6).expect("seeded RNG");
        assert_eq!(value.len(), 12);
    }

    // Sanity check only: ensures bits vary across samples.
    // This is not a statistical entropy test.
    #[test]
    fn test_bit_entropy() {
        seed_from([3; 32]);

        let mut bits = 0u64;
        for _ in 0..100 {
            bits |= next_u64().expect("seeded RNG");
        }

        let bit_count = bits.count_ones();
        assert!(
            bit_count > 8,
            "Low entropy: only {bit_count} bits set in 100 samples",
        );
    }
}
