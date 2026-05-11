use serde::{Serialize, de::DeserializeOwned};
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
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
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

// Write a plain text payload to a requested file or stdout.
pub fn write_text<E>(out: Option<&PathBuf>, text: &str) -> Result<(), E>
where
    E: From<io::Error>,
{
    if let Some(path) = out {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, text)?;
    } else {
        println!("{text}");
    }
    Ok(())
}

// Read and decode one JSON file.
pub fn read_json_file<T, E>(path: &PathBuf) -> Result<T, E>
where
    T: DeserializeOwned,
    E: From<io::Error> + From<serde_json::Error>,
{
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(E::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use serde_json::json;

    // Ensure --out style JSON writes can create nested output directories.
    #[test]
    fn write_pretty_json_creates_parent_directories() {
        let root = temp_dir("canic-cli-output-parent");
        let out = root.join("reports/nested/summary.json");

        write_pretty_json::<_, Box<dyn std::error::Error>>(Some(&out), &json!({"ok": true}))
            .expect("write json");

        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(&out).expect("read json")).expect("parse json");
        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(value["ok"], true);
    }
}
