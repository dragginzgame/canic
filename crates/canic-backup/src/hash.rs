use sha2::{Digest, Sha256};

// Compute lowercase hexadecimal SHA-256 from in-memory bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    digest_hex(Sha256::digest(bytes))
}

// Encode a finalized digest as lowercase hexadecimal.
pub fn digest_hex(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(hex_char(byte >> 4));
        out.push(hex_char(byte & 0x0f));
    }
    out
}

// Convert one four-bit nibble to lowercase hexadecimal.
fn hex_char(nibble: u8) -> char {
    char::from(b"0123456789abcdef"[usize::from(nibble & 0x0f)])
}
