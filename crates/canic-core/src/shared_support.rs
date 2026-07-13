pub mod format {
    pub use crate::format::{byte_size, cycles_tc, truncate};
}

/// Return whether a name uses canonical lowercase ASCII snake_case.
#[must_use]
pub const fn is_ascii_snake_case(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_lowercase() {
        return false;
    }

    let mut index = 1;
    let mut previous_was_underscore = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
            previous_was_underscore = false;
        } else if byte == b'_' && !previous_was_underscore {
            previous_was_underscore = true;
        } else {
            return false;
        }
        index += 1;
    }

    !previous_was_underscore
}
