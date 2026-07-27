//! Module: install_root::operations::activation
//!
//! Responsibility: parse and render exact installed Canister module hashes.
//! Does not own: install commands, activation transitions, or Fleet authority.
//! Boundary: journalled Coordinator and Fleet Subnet Root workflows share this strict projection.

pub(in crate::install_root) fn parse_module_hash(value: &str) -> Option<[u8; 32]> {
    let value = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    if value.len() != 64 {
        return None;
    }
    let mut bytes = [0; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Some(bytes)
}

const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(in crate::install_root) fn module_hash_text(bytes: [u8; 32]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(64), |mut text, byte| {
            use std::fmt::Write as _;
            let _ = write!(text, "{byte:02x}");
            text
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_hash_projection_is_exact_and_prefix_tolerant() {
        let bytes = [0xab; 32];
        let text = "ab".repeat(32);

        assert_eq!(module_hash_text(bytes), text);
        assert_eq!(parse_module_hash(&text), Some(bytes));
        assert_eq!(parse_module_hash(&format!("0x{text}")), Some(bytes));
        assert_eq!(parse_module_hash("not-a-module"), None);
    }
}
