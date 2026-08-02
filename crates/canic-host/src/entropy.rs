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
    #[cfg(unix)]
    {
        use std::{fs::File, io::Read};

        let mut source = File::open("/dev/urandom").map_err(EntropyError::Io)?;
        let mut bytes = [0; 32];
        let mut filled = 0;
        while filled < bytes.len() {
            let current = match source.read(&mut bytes[filled..]) {
                Ok(current) => current,
                Err(source) if source.kind() == io::ErrorKind::Interrupted => continue,
                Err(source) => return Err(EntropyError::Io(source)),
            };
            if current == 0 {
                return Err(EntropyError::ShortRead { actual: filled });
            }
            filled += current;
        }
        Ok(bytes)
    }

    #[cfg(not(unix))]
    {
        Err(EntropyError::Io(io::Error::new(
            io::ErrorKind::Unsupported,
            "cryptographic identity generation requires a Unix entropy source",
        )))
    }
}
