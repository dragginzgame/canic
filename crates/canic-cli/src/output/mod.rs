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
    path::{Path, PathBuf},
};

use ic_host_fs::durable::write_bytes;
use serde::{Serialize, de::DeserializeOwned};

/// Resolve the selected output's existing parent once, including macOS `/tmp`.
/// Missing directories remain for durable publication; the final file cannot be a link.
pub fn resolve_operator_path(path: &Path) -> io::Result<PathBuf> {
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "output target has no file name",
        )
    })?;
    let parent_of = |path: &Path| {
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    };
    let mut parent = parent_of(path);
    let mut missing = Vec::new();
    let mut resolved = loop {
        match fs::canonicalize(&parent) {
            Ok(directory) if directory.is_dir() => break directory,
            Ok(_) => return Err(io::Error::from(io::ErrorKind::NotADirectory)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let Some(name) = parent.file_name() else {
                    return Err(error);
                };
                missing.push(name.to_owned());
                parent = parent_of(&parent);
            }
            Err(error) => return Err(error),
        }
    };
    for directory in missing.into_iter().rev() {
        resolved.push(directory);
    }
    resolved.push(file_name);
    match fs::symlink_metadata(&resolved) {
        Ok(metadata) if !metadata.is_file() => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "output target is not a regular file",
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    Ok(resolved)
}

/// Write a pretty JSON payload to a requested file or stdout.
pub fn write_pretty_json<T, E>(out: Option<&Path>, value: &T) -> Result<(), E>
where
    T: Serialize,
    E: From<io::Error> + From<serde_json::Error>,
{
    if let Some(path) = out {
        let data = serde_json::to_vec_pretty(value)?;
        write_bytes(&resolve_operator_path(path)?, &data)?;
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
    write_bytes(&resolve_operator_path(path)?, &data)?;
    Ok(())
}

/// Write a plain text payload to a requested file or stdout.
pub fn write_text<E>(out: Option<&Path>, text: &str) -> Result<(), E>
where
    E: From<io::Error>,
{
    if let Some(path) = out {
        write_bytes(&resolve_operator_path(path)?, text.as_bytes())?;
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
