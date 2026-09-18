//! Ephemeral native certificate fixtures shared by the Ledger receipt tests.

use ic_certification::{Certificate, HashTree};
use ic_verify_bls_signature::PrivateKey;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn signing_key() -> PrivateKey {
    // Ephemeral native-test key; no key material is stored in the repository.
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).unwrap();
    bytes[0] = 0;
    PrivateKey::deserialize(&bytes).unwrap()
}

pub(super) fn der_key(key: &PrivateKey) -> Vec<u8> {
    let mut der = b"\x30\x81\x82\x30\x1d\x06\x0d\x2b\x06\x01\x04\x01\x82\xdc\x7c\x05\x03\x01\x02\x01\x06\x0c\x2b\x06\x01\x04\x01\x82\xdc\x7c\x05\x03\x02\x01\x03\x61\x00".to_vec();
    der.extend_from_slice(&key.public_key().serialize());
    der
}

pub(super) fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

pub(super) fn sign(tree: HashTree, key: &PrivateKey) -> Certificate {
    let mut message = b"\x0Dic-state-root".to_vec();
    message.extend_from_slice(&tree.digest());
    Certificate {
        tree,
        signature: key.sign(&message).serialize().to_vec(),
        delegation: None,
    }
}

pub(super) fn cbor(value: &impl Serialize) -> Vec<u8> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(value, &mut bytes).unwrap();
    bytes
}

pub(super) fn leb128(mut value: u128) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let byte = u8::try_from(value & 0x7f).unwrap();
        value >>= 7;
        bytes.push(if value == 0 { byte } else { byte | 0x80 });
        if value == 0 {
            return bytes;
        }
    }
}
