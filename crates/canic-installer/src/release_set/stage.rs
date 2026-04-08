use canic::protocol;
use canic_core::CANIC_WASM_CHUNK_BYTES;
use flate2::read::GzDecoder;
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use std::{
    fmt::Write,
    fs,
    io::{Read, Write as IoWrite},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    GZIP_MAGIC, ReleaseSetEntry, RootReleaseSetManifest, WASM_MAGIC, dfx_root, root_time_secs,
};

// Stage one emitted release-set manifest into root and resume bootstrap-ready state.
pub fn stage_root_release_set(
    dfx_root: &Path,
    root_canister: &str,
    manifest: &RootReleaseSetManifest,
) -> Result<(), Box<dyn std::error::Error>> {
    let now_secs = root_time_secs(root_canister)?;
    let total_entries = manifest.entries.len();

    print_stage_progress(&format!(
        "Staging {total_entries} release entries into {root_canister}"
    ));

    for (entry_index, entry) in manifest.entries.iter().enumerate() {
        stage_release_entry(
            dfx_root,
            root_canister,
            &manifest.release_version,
            entry,
            now_secs,
            entry_index + 1,
            total_entries,
        )?;
    }

    print_stage_progress(&format!(
        "Finished staging {total_entries} release entries into {root_canister}"
    ));
    Ok(())
}

// Trigger root bootstrap resume after the ordinary release set is fully staged.
pub fn resume_root_bootstrap(root_canister: &str) -> Result<(), Box<dyn std::error::Error>> {
    let _ = dfx_call(
        root_canister,
        protocol::CANIC_WASM_STORE_BOOTSTRAP_RESUME_ROOT_ADMIN,
        None,
        None,
    )?;
    Ok(())
}

// Run one `dfx canister call` and return stdout, preserving stderr on failure.
pub fn dfx_call(
    canister: &str,
    method: &str,
    argument: Option<&str>,
    output: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let dfx_root = dfx_root()?;
    let mut command = Command::new("dfx");
    command.current_dir(&dfx_root);
    command.args(["canister", "call", canister, method]);

    if let Some(output) = output {
        command.args(["--output", output]);
    }

    let temp_argument_path = argument.map(write_argument_file).transpose()?;
    if let Some(path) = temp_argument_path.as_ref() {
        command.arg("--argument-file").arg(path);
    }

    let result = command.output()?;

    if let Some(path) = temp_argument_path {
        let _ = fs::remove_file(path);
    }

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        return Err(format!(
            "dfx canister call {} {} failed: {}\n{}",
            canister,
            method,
            result.status,
            if stderr.trim().is_empty() {
                stdout.trim()
            } else {
                stderr.trim()
            }
        )
        .into());
    }

    let stdout = String::from_utf8(result.stdout)?;
    Ok(stdout)
}

// Compute the canonical SHA-256 hash used by the template staging APIs.
#[must_use]
pub fn wasm_hash(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().to_vec()
}

// Compute the canonical SHA-256 hash and render it as lowercase hex.
#[must_use]
pub fn wasm_hash_hex(bytes: &[u8]) -> String {
    hex_bytes(&wasm_hash(bytes))
}

// Encode one string as a Candid text literal.
#[must_use]
pub fn idl_text(value: &str) -> String {
    serde_json::to_string(value).expect("string literal encoding must succeed")
}

// Encode one blob as a Candid text blob literal.
#[must_use]
pub fn idl_blob(bytes: &[u8]) -> String {
    let mut encoded = String::from("blob \"");

    for byte in bytes {
        let _ = write!(encoded, "\\{byte:02X}");
    }

    encoded.push('"');
    encoded
}

// Decode a JSON nat that may be emitted as either a number or a string.
#[must_use]
pub fn json_u64(value: &JsonValue) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|raw| raw.parse::<u64>().ok()))
}

// Build one release-set entry from one built ordinary role artifact.
pub(super) fn build_release_set_entry(
    dfx_root: &Path,
    artifact_root: &Path,
    role_name: &str,
) -> Result<ReleaseSetEntry, Box<dyn std::error::Error>> {
    let artifact_path = artifact_root
        .join(role_name)
        .join(format!("{role_name}.wasm.gz"));
    let artifact_relative_path = artifact_path
        .strip_prefix(dfx_root)
        .map_err(|_| {
            format!(
                "artifact {} is not under DFX root {}",
                artifact_path.display(),
                dfx_root.display()
            )
        })?
        .to_string_lossy()
        .to_string();
    let wasm_module = read_release_artifact(&artifact_path)?;

    let chunk_hashes = wasm_module
        .chunks(CANIC_WASM_CHUNK_BYTES)
        .map(wasm_hash_hex)
        .collect::<Vec<_>>();

    Ok(ReleaseSetEntry {
        role: role_name.to_string(),
        template_id: format!("embedded:{role_name}"),
        artifact_relative_path,
        payload_size_bytes: wasm_module.len() as u64,
        payload_sha256_hex: wasm_hash_hex(&wasm_module),
        chunk_size_bytes: CANIC_WASM_CHUNK_BYTES as u64,
        chunk_sha256_hex: chunk_hashes,
    })
}

// Stage one manifest, prepare its chunk set, and publish all chunk bytes into root.
fn stage_release_entry(
    dfx_root: &Path,
    root_canister: &str,
    release_version: &str,
    entry: &ReleaseSetEntry,
    now_secs: u64,
    entry_index: usize,
    total_entries: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let artifact_path = dfx_root.join(&entry.artifact_relative_path);
    let wasm_module = read_release_artifact(&artifact_path)?;

    if wasm_module.len() as u64 != entry.payload_size_bytes {
        return Err(format!(
            "release artifact size drift for {}: manifest={} actual={} ({})",
            entry.role,
            entry.payload_size_bytes,
            wasm_module.len(),
            artifact_path.display()
        )
        .into());
    }

    let chunk_count = wasm_module.chunks(CANIC_WASM_CHUNK_BYTES).count();
    if chunk_count != entry.chunk_sha256_hex.len() {
        return Err(format!(
            "release chunk count drift for {}: manifest={} actual={} ({})",
            entry.role,
            entry.chunk_sha256_hex.len(),
            chunk_count,
            artifact_path.display()
        )
        .into());
    }
    let payload_hash = decode_hex(&entry.payload_sha256_hex)?;

    print_stage_progress(&format!(
        "Staging release {entry_index}/{total_entries}: {} ({} chunk{})",
        entry.role,
        chunk_count,
        if chunk_count == 1 { "" } else { "s" }
    ));

    stage_release_manifest(
        root_canister,
        release_version,
        entry,
        now_secs,
        &payload_hash,
        wasm_module.len(),
    )?;
    print_stage_progress(&format!(
        "Staged manifest for {} ({entry_index}/{total_entries})",
        entry.role
    ));

    prepare_release_chunks(
        root_canister,
        release_version,
        entry,
        &payload_hash,
        wasm_module.len(),
    )?;
    print_stage_progress(&format!(
        "Prepared chunk upload for {} ({}/{})",
        entry.role, entry_index, total_entries
    ));

    publish_release_chunks(root_canister, release_version, entry, &wasm_module)?;

    print_stage_progress(&format!(
        "Finished release {entry_index}/{total_entries}: {}",
        entry.role
    ));
    Ok(())
}

// Stage one approved manifest into root before any chunk preparation/upload begins.
fn stage_release_manifest(
    root_canister: &str,
    release_version: &str,
    entry: &ReleaseSetEntry,
    now_secs: u64,
    payload_hash: &[u8],
    payload_size_bytes: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = format!(
        "(record {{ template_id = {}; role = {}; version = {}; payload_hash = {}; \
         payload_size_bytes = {} : nat64; store_binding = \"bootstrap\"; \
         chunking_mode = variant {{ Chunked }}; manifest_state = variant {{ Approved }}; \
         approved_at = opt ({} : nat64); created_at = {} : nat64 }})",
        idl_text(&entry.template_id),
        idl_text(&entry.role),
        idl_text(release_version),
        idl_blob(payload_hash),
        payload_size_bytes,
        now_secs,
        now_secs,
    );
    let _ = dfx_call(
        root_canister,
        protocol::CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
        Some(&manifest),
        None,
    )?;
    Ok(())
}

// Prepare the root-local chunk set metadata before sending any chunk bytes.
fn prepare_release_chunks(
    root_canister: &str,
    release_version: &str,
    entry: &ReleaseSetEntry,
    payload_hash: &[u8],
    payload_size_bytes: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let chunk_hash_literals = entry
        .chunk_sha256_hex
        .iter()
        .map(|hash| decode_hex(hash).map(|bytes| idl_blob(&bytes)))
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?
        .join("; ");

    let prepare = format!(
        "(record {{ template_id = {}; version = {}; payload_hash = {}; \
         payload_size_bytes = {} : nat64; chunk_hashes = vec {{ {} }} }})",
        idl_text(&entry.template_id),
        idl_text(release_version),
        idl_blob(payload_hash),
        payload_size_bytes,
        chunk_hash_literals,
    );
    let _ = dfx_call(
        root_canister,
        protocol::CANIC_TEMPLATE_PREPARE_ADMIN,
        Some(&prepare),
        None,
    )?;
    Ok(())
}

// Upload every prepared chunk and print live progress before and after each call.
fn publish_release_chunks(
    root_canister: &str,
    release_version: &str,
    entry: &ReleaseSetEntry,
    wasm_module: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let total_chunks = entry.chunk_sha256_hex.len();

    for (chunk_index, chunk) in wasm_module.chunks(CANIC_WASM_CHUNK_BYTES).enumerate() {
        let chunk_number = chunk_index + 1;
        print_stage_progress(&format!(
            "Uploading chunk {chunk_number}/{total_chunks} for {} ({} bytes)",
            entry.role,
            chunk.len()
        ));
        let request = format!(
            "(record {{ template_id = {}; version = {}; chunk_index = {} : nat32; bytes = {} }})",
            idl_text(&entry.template_id),
            idl_text(release_version),
            chunk_index,
            idl_blob(chunk),
        );
        let _ = dfx_call(
            root_canister,
            protocol::CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
            Some(&request),
            None,
        )?;
        print_stage_progress(&format!(
            "Uploaded chunk {chunk_number}/{total_chunks} for {}",
            entry.role
        ));
    }
    Ok(())
}

// Print one installer progress line immediately so long staging loops stay visible.
fn print_stage_progress(message: &str) {
    println!("{message}");
    let _ = std::io::stdout().flush();
}

// Read one staged release artifact and validate that it is a non-empty gzip stream
// whose decompressed payload is a real wasm module.
pub(super) fn read_release_artifact(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
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

// Persist one temporary Candid argument file for `dfx --argument-file`.
fn write_argument_file(argument: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!(
        "canic-stage-root-release-set-{}-{unique}.did",
        std::process::id()
    ));
    fs::write(&path, argument)?;
    Ok(path)
}

// Render one byte slice as lowercase hexadecimal.
fn hex_bytes(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        let _ = write!(encoded, "{byte:02x}");
    }

    encoded
}

// Decode one lowercase hex string back into bytes.
fn decode_hex(hex: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if !hex.len().is_multiple_of(2) {
        return Err(format!("invalid hex length: {}", hex.len()).into());
    }

    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for index in (0..hex.len()).step_by(2) {
        bytes.push(u8::from_str_radix(&hex[index..index + 2], 16)?);
    }

    Ok(bytes)
}
