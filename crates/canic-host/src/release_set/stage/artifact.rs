use super::super::{GZIP_MAGIC, ReleaseSetEntry, WASM_MAGIC};
use canic_core::{CANIC_WASM_CHUNK_BYTES, cdk::utils::hash::wasm_hash_hex};
use flate2::read::GzDecoder;
use std::{fs, io::Read, path::Path};

// Build one release-set entry from one built ordinary role artifact.
pub(in crate::release_set) fn build_release_set_entry(
    icp_root: &Path,
    artifact_root: &Path,
    role_name: &str,
) -> Result<ReleaseSetEntry, Box<dyn std::error::Error>> {
    let artifact_path = artifact_root
        .join(role_name)
        .join(format!("{role_name}.wasm.gz"));
    let artifact_relative_path = artifact_path
        .strip_prefix(icp_root)
        .map_err(|_| {
            format!(
                "artifact {} is not under ICP root {}",
                artifact_path.display(),
                icp_root.display()
            )
        })?
        .to_string_lossy()
        .to_string();
    let artifact = read_release_artifact(&artifact_path)?;

    let chunk_hashes = artifact
        .chunks(CANIC_WASM_CHUNK_BYTES)
        .map(wasm_hash_hex)
        .collect::<Vec<_>>();

    Ok(ReleaseSetEntry {
        role: role_name.to_string(),
        template_id: format!("embedded:{role_name}"),
        artifact_relative_path,
        payload_size_bytes: u64::try_from(artifact.len())?,
        payload_sha256_hex: wasm_hash_hex(&artifact),
        chunk_size_bytes: u64::try_from(CANIC_WASM_CHUNK_BYTES)?,
        chunk_sha256_hex: chunk_hashes,
    })
}

// Read one staged release artifact and validate that it is a non-empty gzip stream
// whose decompressed payload is a real wasm module.
pub(in crate::release_set) fn read_release_artifact(
    path: &Path,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let artifact = fs::read(path)?;

    if artifact.is_empty() {
        return Err(format!("release artifact is empty: {}", path.display()).into());
    }

    if !artifact.starts_with(&GZIP_MAGIC) {
        return Err(format!(
            "release artifact is not gzip-compressed: {}",
            path.display()
        )
        .into());
    }

    let mut decoder = GzDecoder::new(&artifact[..]);
    let mut wasm = Vec::new();
    decoder
        .read_to_end(&mut wasm)
        .map_err(|err| format!("failed to decompress {}: {err}", path.display()))?;

    if wasm.is_empty() {
        return Err(format!(
            "release artifact decompresses to zero bytes: {}",
            path.display()
        )
        .into());
    }

    if !wasm.starts_with(&WASM_MAGIC) {
        return Err(format!(
            "release artifact does not decompress to a wasm module: {}",
            path.display()
        )
        .into());
    }

    Ok(artifact)
}
