//! Module: canic_cli::output
//!
//! Responsibility: share small CLI output file/stdout helpers.
//! Does not own: command-specific report formats, serialization schema, or diagnostics.
//! Boundary: writes caller-provided text/JSON payloads to stdout or filesystem paths.

#[cfg(test)]
mod tests;

use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs,
    io::{self, Write},
    path::Path,
};

/// Write a pretty JSON payload to a requested file or stdout.
pub fn write_pretty_json<T, E>(out: Option<&Path>, value: &T) -> Result<(), E>
where
    T: Serialize,
    E: From<io::Error> + From<serde_json::Error>,
{
    if let Some(path) = out {
        ensure_parent_dir::<E>(path)?;
        let data = serde_json::to_vec_pretty(value)?;
        fs::write(path, data)?;
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
    ensure_parent_dir::<E>(path)?;
    let data = serde_json::to_vec_pretty(value)?;
    fs::write(path, data)?;
    Ok(())
}

/// Write a plain text payload to a requested file or stdout.
pub fn write_text<E>(out: Option<&Path>, text: &str) -> Result<(), E>
where
    E: From<io::Error>,
{
    if let Some(path) = out {
        ensure_parent_dir::<E>(path)?;
        fs::write(path, text)?;
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

fn ensure_parent_dir<E>(path: &Path) -> Result<(), E>
where
    E: From<io::Error>,
{
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}
