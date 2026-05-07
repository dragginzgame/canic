use flate2::{Compression, GzBuilder};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    process::Command,
};

// Apply `ic-wasm shrink` when available; absence of the optional tool is not
// fatal, but execution failures are surfaced because they usually mean bad IO.
pub fn maybe_shrink_wasm_artifact(wasm_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let shrunk_path = wasm_path.with_extension("wasm.shrunk");
    match Command::new("ic-wasm")
        .arg(wasm_path)
        .arg("-o")
        .arg(&shrunk_path)
        .arg("shrink")
        .status()
    {
        Ok(status) if status.success() => {
            fs::rename(shrunk_path, wasm_path)?;
        }
        Ok(_) => {
            let _ = fs::remove_file(shrunk_path);
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            return Err(format!("failed to run ic-wasm for {}: {err}", wasm_path.display()).into());
        }
    }

    Ok(())
}

// Copy one `.wasm` artifact atomically into the DFX artifact tree.
pub fn write_wasm_artifact(
    source_path: &Path,
    target_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(source_path)?;
    write_bytes_atomically(target_path, &bytes)?;
    Ok(())
}

// Write one deterministic `.wasm.gz` artifact with a zeroed gzip timestamp.
pub fn write_gzip_artifact(
    wasm_path: &Path,
    wasm_gz_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut wasm_bytes = Vec::new();
    fs::File::open(wasm_path)?.read_to_end(&mut wasm_bytes)?;

    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder.write_all(&wasm_bytes)?;
    let gz_bytes = encoder.finish()?;
    write_bytes_atomically(wasm_gz_path, &gz_bytes)?;
    Ok(())
}

// Persist one file through a sibling temp path so readers never observe a partial write.
pub fn write_bytes_atomically(
    target_path: &Path,
    bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let tmp_path = target_path.with_extension(format!(
        "{}.tmp",
        target_path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
    ));
    fs::write(&tmp_path, bytes)?;
    fs::rename(tmp_path, target_path)?;
    Ok(())
}
