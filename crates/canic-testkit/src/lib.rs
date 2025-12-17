pub mod pic;

use canic::cdk::types::{Account, Principal};

///
/// Fake
///

pub struct Fake;

impl Fake {
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
