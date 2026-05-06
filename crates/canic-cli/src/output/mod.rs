use serde::Serialize;
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

// Write a pretty JSON payload to a requested file or stdout.
pub fn write_pretty_json<T, E>(out: Option<&PathBuf>, value: &T) -> Result<(), E>
where
    T: Serialize,
    E: From<io::Error> + From<serde_json::Error>,
{
    if let Some(path) = out {
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

// Write a pretty JSON artifact file, creating its parent directory when needed.
pub fn write_pretty_json_file<T, E>(path: &PathBuf, value: &T) -> Result<(), E>
where
    T: Serialize,
    E: From<io::Error> + From<serde_json::Error>,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let data = serde_json::to_vec_pretty(value)?;
    fs::write(path, data)?;
    Ok(())
}
