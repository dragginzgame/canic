//! Module: canic_cli::output
//!
//! Responsibility: share small CLI output file/stdout helpers.
//! Does not own: command-specific report formats, serialization schema, or diagnostics.
//! Boundary: writes caller-provided text/JSON payloads to stdout or filesystem paths.

#[cfg(test)]
mod tests;

use std::{
    fs,
    io::{self, Write},
    path::Path,
};

use canic_host::durable_io::write_bytes;
use serde::{Serialize, de::DeserializeOwned};

/// Write a pretty JSON payload to a requested file or stdout.
pub fn write_pretty_json<T, E>(out: Option<&Path>, value: &T) -> Result<(), E>
where
    T: Serialize,
    E: From<io::Error> + From<serde_json::Error>,
{
    if let Some(path) = out {
        let data = serde_json::to_vec_pretty(value)?;
        write_bytes(path, &data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, value)?;
    writeln!(handle)?;
    Ok(())
}

/// Write a pretty JSON artifact file, creating its parent directory when needed.
pub fn write_pretty_json_file<T, E>(path: &Path, value: &T) -> Result<(), E>
where
    T: Serialize,
    E: From<io::Error> + From<serde_json::Error>,
{
    let data = serde_json::to_vec_pretty(value)?;
    write_bytes(path, &data)?;
    Ok(())
}

/// Write a plain text payload to a requested file or stdout.
pub fn write_text<E>(out: Option<&Path>, text: &str) -> Result<(), E>
where
    E: From<io::Error>,
{
    if let Some(path) = out {
        write_bytes(path, text.as_bytes())?;
    } else {
        println!("{text}");
    }
    Ok(())
}

/// Read and decode one JSON file.
pub fn read_json_file<T, E>(path: &Path) -> Result<T, E>
where
    T: DeserializeOwned,
    E: From<io::Error> + From<serde_json::Error>,
{
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(E::from)
}
