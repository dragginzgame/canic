use canic::types::{Account, Principal, Ulid};

pub mod pic;

pub struct Fake;

impl Fake {
    #[must_use]
    pub fn account(seed: usize) -> Account {
        let mut buf = [0u8; 32];
        buf[..4].copy_from_slice(&seed.to_be_bytes());

        Account {
            owner: Self::principal(seed),
            subaccount: Some(buf),
        }
    }

    #[must_use]
    pub fn principal(seed: usize) -> Principal {
        let mut buf = [0u8; 29];
        buf[..4].copy_from_slice(&seed.to_be_bytes());

        Principal::from_slice(&buf)
    }

    #[must_use]
    pub fn ulid(seed: usize) -> Ulid {
        let mut buf = [0u8; 16];
        buf[..4].copy_from_slice(&seed.to_be_bytes());

        Ulid::from_bytes(buf)
    }
}
