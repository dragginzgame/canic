use canic::types::{Account, Principal, Ulid};

pub mod pic;

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

    #[must_use]
    pub fn ulid(seed: u32) -> Ulid {
        let mut buf = [0u8; 16];
        buf[..4].copy_from_slice(&seed.to_be_bytes());

        Ulid::from_bytes(buf)
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

    #[test]
    fn fake_ulid_is_deterministic_and_unique() {
        let u1 = Fake::ulid(1234);
        let u2 = Fake::ulid(1234);
        let v = Fake::ulid(5678);

        assert_eq!(u1, u2, "Fake::ulid should be deterministic");
        assert_ne!(u1, v, "Fake::ulid should differ for different seeds");

        let bytes = u1.to_bytes();
        assert_eq!(bytes.len(), 16, "ULID must be 16 bytes");
    }
}
