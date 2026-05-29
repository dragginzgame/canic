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

// Copy one `.wasm` artifact atomically into the local ICP artifact tree.
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

// Embed the extracted service interface for local artifacts so
// `icp canister metadata <canister> candid:service` introspection works during
// development. Production `ic` builds skip this path.
pub fn embed_candid_metadata(
    wasm_path: &Path,
    did_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    embed_candid_metadata_with_command("ic-wasm", wasm_path, did_path)
}

fn embed_candid_metadata_with_command(
    command_name: &str,
    wasm_path: &Path,
    did_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(command_name)
        .arg(wasm_path)
        .args(["-o"])
        .arg(wasm_path)
        .args(["metadata", "candid:service", "-f"])
        .arg(did_path)
        .args(["-v", "public"])
        .output();

    let output = match output {
        Ok(output) => output,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(format!(
                "failed to run ic-wasm metadata for {}: {err}",
                wasm_path.display()
            )
            .into());
        }
    };

    if !output.status.success() {
        return Err(format!(
            "ic-wasm metadata failed for {}: {}",
            wasm_path.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn missing_ic_wasm_metadata_tool_is_nonfatal() {
        let root = unique_temp_dir("canic-missing-ic-wasm-metadata");
        fs::create_dir_all(&root).expect("create temp dir");
        let wasm_path = root.join("test.wasm");
        let did_path = root.join("test.did");
        fs::write(&wasm_path, b"\0asm").expect("write wasm placeholder");
        fs::write(&did_path, b"service : {}").expect("write did placeholder");

        let missing_tool = root.join("missing-ic-wasm");
        embed_candid_metadata_with_command(
            &missing_tool.display().to_string(),
            &wasm_path,
            &did_path,
        )
        .expect("missing ic-wasm should not fail metadata embedding");

        fs::remove_dir_all(root).expect("remove temp dir");
    }

    fn unique_temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{label}-{}-{nanos}", std::process::id()))
    }
}
