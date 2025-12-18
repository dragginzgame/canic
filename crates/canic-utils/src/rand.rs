//!
//! Thread-local, non-cryptographic RNG seeded from wall-clock time.
//!
//! The IC executes canister code single-threaded, so `RefCell` provides
//! sufficient interior mutability without locking or poisoning semantics.
//!
use canic_cdk::utils::time::now_nanos;
use std::cell::RefCell;
use tinyrand::{Rand, Seeded, StdRand};

thread_local! {
    static STD_RAND: RefCell<StdRand> = RefCell::new(StdRand::seed(now_nanos()));
}

/// Produce an 8-bit random value (derived from `next_u16`).
#[must_use]
pub fn next_u8() -> u8 {
    (next_u16() & 0xFF) as u8
}

/// Produce a 16-bit random value from the shared RNG.
#[must_use]
pub fn next_u16() -> u16 {
    STD_RAND.with(|rng| rng.borrow_mut().next_u16())
}

/// Produce a 32-bit random value from the shared RNG.
#[must_use]
pub fn next_u32() -> u32 {
    STD_RAND.with(|rng| rng.borrow_mut().next_u32())
}

/// Produce a 64-bit random value from the shared RNG.
#[must_use]
pub fn next_u64() -> u64 {
    STD_RAND.with(|rng| rng.borrow_mut().next_u64())
}

/// Produce a 128-bit random value from the shared RNG.
#[must_use]
pub fn next_u128() -> u128 {
    STD_RAND.with(|rng| rng.borrow_mut().next_u128())
}

//
// TESTS
//

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_u64s() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        while set.len() < 1000 {
            let random_value = next_u64();
            assert!(set.insert(random_value), "value already in set");
        }
    }

    #[test]
    fn test_rng_reseeding() {
        let mut rng1 = StdRand::seed(now_nanos());
        let mut rng2 = StdRand::seed(now_nanos() + 1);

        let mut matched = false;
        for _ in 0..100 {
            if rng1.next_u64() == rng2.next_u64() {
                matched = true;
                break;
            }
        }
        assert!(
            !matched,
            "RNGs with different seeds unexpectedly produced the same value"
        );
    }

    #[test]
    fn test_determinism_with_fixed_seed() {
        let seed = 42;
        let mut rng1 = StdRand::seed(seed);
        let mut rng2 = StdRand::seed(seed);

        for _ in 0..100 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    // Sanity check only: ensures bits vary across samples.
    // This is not a statistical entropy test.
    #[test]
    fn test_bit_entropy() {
        let mut bits = 0u64;
        for _ in 0..100 {
            bits |= next_u64();
        }

        let bit_count = bits.count_ones();
        assert!(
            bit_count > 8,
            "Low entropy: only {bit_count} bits set in 100 samples",
        );
    }
}
