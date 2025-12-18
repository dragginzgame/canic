//! Test utilities and fixtures for exercising Canic canisters.
//!
//! This crate is intended for host-side test environments (for example via
//! PocketIC) and provides small helpers for generating stable dummy principals
//! and accounts.

pub mod pic;

use canic::cdk::types::{Account, Principal};

///
/// Deterministic dummy-value generator for tests.
///
/// Produces stable principals/accounts derived from a numeric seed, which makes
/// tests reproducible without hardcoding raw byte arrays.
///

pub struct Fake;

impl Fake {
    ///
    /// Deterministically derive an [`Account`] from `seed`.
    ///
    #[must_use]
    pub fn account(seed: u32) -> Account {
        let mut sub = [0u8; 32];
        let bytes = seed.to_be_bytes();
        sub[..4].copy_from_slice(&bytes);

        Account {
            owner: Self::principal(seed),
            subaccount: Some(sub),
        }
    }

    ///
    /// Deterministically derive a [`Principal`] from `seed`.
    ///
    #[must_use]
    pub fn principal(seed: u32) -> Principal {
        let mut buf = [0u8; 29];
        buf[..4].copy_from_slice(&seed.to_be_bytes());

        Principal::from_slice(&buf)
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_account_is_deterministic_and_unique() {
        let a1 = Fake::account(42);
        let a2 = Fake::account(42);
        let b = Fake::account(99);

        // Deterministic: same seed => same account
        assert_eq!(a1, a2, "Fake::account should be deterministic");

        // Unique: different seeds => different account
        assert_ne!(a1, b, "Fake::account should vary by seed");
    }

    #[test]
    fn fake_principal_is_deterministic_and_unique() {
        let p1 = Fake::principal(7);
        let p2 = Fake::principal(7);
        let q = Fake::principal(8);

        assert_eq!(p1, p2, "Fake::principal should be deterministic");
        assert_ne!(p1, q, "Fake::principal should differ for different seeds");

        let bytes = p1.as_slice();
        assert_eq!(bytes.len(), 29, "Principal must be 29 bytes");
    }
}
