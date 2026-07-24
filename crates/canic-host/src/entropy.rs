//! Module: entropy
//!
//! Responsibility: read exact cryptographic identity bytes from the operating system.
//! Does not own: identity derivation, durable publication, collision handling, or retry policy.
//! Boundary: callers assign meaning only after the returned bytes are durably recorded.

use std::io;

///
/// EntropyError
///

#[derive(Debug)]
pub enum EntropyError {
    Io(io::Error),
    ShortRead { actual: usize },
}

/// Read one exact 32-byte cryptographic value from the operating system.
pub fn random_bytes_32() -> Result<[u8; 32], EntropyError> {
    #[cfg(not(windows))]
    {
        use rustix::rand::{GetRandomFlags, getrandom};

        let mut bytes = [0; 32];
        let mut filled = 0;
        while filled < bytes.len() {
            let current =
                getrandom(&mut bytes[filled..], GetRandomFlags::empty()).map_err(|source| {
                    EntropyError::Io(io::Error::from_raw_os_error(source.raw_os_error()))
                })?;
            if current == 0 {
                return Err(EntropyError::ShortRead { actual: filled });
            }
            filled += current;
        }
        Ok(bytes)
    }

    #[cfg(windows)]
    {
        Err(EntropyError::Io(io::Error::new(
            io::ErrorKind::Unsupported,
            "cryptographic identity generation is unsupported on Windows",
        )))
    }
}
