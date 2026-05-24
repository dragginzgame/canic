use sha2::{Digest, Sha256};

#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex_bytes(Sha256::digest(bytes))
}

#[must_use]
pub fn hex_bytes(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut encoded = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        encoded.push(hex_char(byte >> 4));
        encoded.push(hex_char(byte & 0x0f));
    }

    encoded
}

fn hex_char(nibble: u8) -> char {
    char::from(b"0123456789abcdef"[usize::from(nibble & 0x0f)])
}
