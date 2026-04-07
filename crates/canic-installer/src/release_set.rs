use canic::protocol;
use canic_core::{CANIC_WASM_CHUNK_BYTES, bootstrap::parse_config_model};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fmt::Write,
    fs,
    io::{Read, Write as IoWrite},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use toml::Value as TomlValue;

const CANISTERS_ROOT_RELATIVE: &str = "canisters";
const ROOT_CONFIG_FILE: &str = "canic.toml";
const WORKSPACE_MANIFEST_RELATIVE: &str = "Cargo.toml";
const DFX_CONFIG_FILE: &str = "dfx.json";
pub const ROOT_RELEASE_SET_MANIFEST_FILE: &str = "root.release-set.json";
const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];
const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];

#[derive(Clone, Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoMetadataPackage>,
}

#[derive(Clone, Debug, Deserialize)]
struct CargoMetadataPackage {
    name: String,
    manifest_path: PathBuf,
    metadata: Option<JsonValue>,
}

///
/// RootReleaseSetManifest
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootReleaseSetManifest {
    pub release_version: String,
    pub entries: Vec<ReleaseSetEntry>,
}

///
/// ReleaseSetEntry
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReleaseSetEntry {
    pub role: String,
    pub template_id: String,
    pub artifact_relative_path: String,
    pub payload_size_bytes: u64,
    pub payload_sha256_hex: String,
    pub chunk_size_bytes: u64,
    pub chunk_sha256_hex: Vec<String>,
}

// Resolve the downstream Cargo workspace root from the current directory,
// config hints, or an explicit override.
pub fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("CANIC_WORKSPACE_ROOT") {
        return Ok(PathBuf::from(path).canonicalize()?);
    }

    if let Some(root) = std::env::var_os("CANIC_WORKSPACE_MANIFEST_PATH")
        .map(PathBuf::from)
        .and_then(|path| discover_workspace_root_from(&path))
    {
        return Ok(root);
    }

    if let Some(root) = std::env::var_os("CANIC_CONFIG_PATH")
        .map(PathBuf::from)
        .and_then(|path| discover_workspace_root_from(&path))
    {
        return Ok(root);
    }

    if let Some(root) = discover_workspace_root_from(&std::env::current_dir()?) {
        return Ok(root);
    }

    Ok(std::env::current_dir()?.canonicalize()?)
}

// Resolve the downstream DFX/project root from the current directory or an
// explicit override.
pub fn dfx_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("CANIC_DFX_ROOT") {
        return Ok(PathBuf::from(path).canonicalize()?);
    }

    let current_dir = std::env::current_dir()?.canonicalize()?;
    if let Some(root) = discover_dfx_root_from(&current_dir) {
        return Ok(root);
    }

    if let Ok(path) = std::env::var("CANIC_WORKSPACE_ROOT") {
        let workspace_root = PathBuf::from(path).canonicalize()?;
        if let Some(root) = discover_dfx_root_from(&workspace_root) {
            return Ok(root);
        }
        return Ok(workspace_root);
    }

    Ok(current_dir)
}

// Resolve the downstream Canic config path.
#[must_use]
pub fn config_path(workspace_root: &Path) -> PathBuf {
    std::env::var_os("CANIC_CONFIG_PATH").map_or_else(
        || canisters_root(workspace_root).join(ROOT_CONFIG_FILE),
        |path| normalize_workspace_path(workspace_root, PathBuf::from(path)),
    )
}

// Resolve the downstream canister-manifest root.
#[must_use]
pub fn canisters_root(workspace_root: &Path) -> PathBuf {
    if let Some(path) = std::env::var_os("CANIC_CANISTERS_ROOT") {
        return normalize_workspace_path(workspace_root, PathBuf::from(path));
    }

    if let Some(path) = std::env::var_os("CANIC_CONFIG_PATH") {
        let config_path = normalize_workspace_path(workspace_root, PathBuf::from(path));
        if let Some(parent) = config_path.parent() {
            return parent.to_path_buf();
        }
    }

    if let Some(manifest_path) = discover_canister_manifest_from_metadata(workspace_root, "root")
        && let Some(parent) = manifest_path.parent().and_then(Path::parent)
    {
        return parent.to_path_buf();
    }

    workspace_root.join(CANISTERS_ROOT_RELATIVE)
}

// Resolve the downstream root canister manifest path.
#[must_use]
pub fn root_manifest_path(workspace_root: &Path) -> PathBuf {
    std::env::var_os("CANIC_ROOT_MANIFEST_PATH").map_or_else(
        || {
            discover_canister_manifest_from_metadata(workspace_root, "root").unwrap_or_else(|| {
                canisters_root(workspace_root)
                    .join("root")
                    .join("Cargo.toml")
            })
        },
        |path| normalize_workspace_path(workspace_root, PathBuf::from(path)),
    )
}

// Resolve the downstream manifest path for one visible canister role.
#[must_use]
pub fn canister_manifest_path(workspace_root: &Path, canister_name: &str) -> PathBuf {
    discover_canister_manifest_from_metadata(workspace_root, canister_name).unwrap_or_else(|| {
        canisters_root(workspace_root)
            .join(canister_name)
            .join("Cargo.toml")
    })
}

// Resolve the downstream workspace manifest path.
#[must_use]
pub fn workspace_manifest_path(workspace_root: &Path) -> PathBuf {
    std::env::var_os("CANIC_WORKSPACE_MANIFEST_PATH").map_or_else(
        || workspace_root.join(WORKSPACE_MANIFEST_RELATIVE),
        |path| normalize_workspace_path(workspace_root, PathBuf::from(path)),
    )
}

// Prefer the selected DFX network artifact root and fall back to local when present.
pub fn resolve_artifact_root(
    dfx_root: &Path,
    network: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let preferred = dfx_root.join(".dfx").join(network).join("canisters");
    if preferred.is_dir() {
        return Ok(preferred);
    }

    let fallback = dfx_root.join(".dfx/local/canisters");
    if fallback.is_dir() {
        return Ok(fallback);
    }

    Err(format!(
        "missing built DFX artifacts under {} or {}",
        preferred.display(),
        fallback.display()
    )
    .into())
}

// Return the canonical manifest path for the staged root release set.
pub fn root_release_set_manifest_path(
    artifact_root: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let manifest_path = artifact_root
        .join("root")
        .join(ROOT_RELEASE_SET_MANIFEST_FILE);

    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)?;
    }

    Ok(manifest_path)
}

// Build and persist the current root release-set manifest from built `.wasm.gz` artifacts.
pub fn emit_root_release_set_manifest(
    workspace_root: &Path,
    dfx_root: &Path,
    network: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let artifact_root = resolve_artifact_root(dfx_root, network)?;
    let config_path = config_path(workspace_root);
    let manifest_path = root_release_set_manifest_path(&artifact_root)?;
    let release_version = load_root_package_version(
        &root_manifest_path(workspace_root),
        &workspace_manifest_path(workspace_root),
    )?;
    let entries = configured_release_roles(&config_path)?
        .into_iter()
        .map(|role_name| build_release_set_entry(dfx_root, &artifact_root, &role_name))
        .collect::<Result<Vec<_>, _>>()?;
    let manifest = RootReleaseSetManifest {
        release_version,
        entries,
    };

    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(manifest_path)
}

// Emit the root release-set manifest only once every required ordinary artifact exists.
pub fn emit_root_release_set_manifest_if_ready(
    workspace_root: &Path,
    dfx_root: &Path,
    network: &str,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let artifact_root = resolve_artifact_root(dfx_root, network)?;
    let roles = configured_release_roles(&config_path(workspace_root))?;

    for role_name in roles {
        let artifact_path = artifact_root
            .join(&role_name)
            .join(format!("{role_name}.wasm.gz"));
        if !artifact_path.is_file() {
            return Ok(None);
        }
    }

    emit_root_release_set_manifest(workspace_root, dfx_root, network).map(Some)
}

// Load one previously emitted root release-set manifest from disk.
pub fn load_root_release_set_manifest(
    manifest_path: &Path,
) -> Result<RootReleaseSetManifest, Box<dyn std::error::Error>> {
    let source = fs::read(manifest_path)?;
    Ok(serde_json::from_slice(&source)?)
}

// Enumerate the configured ordinary roles that root must publish before bootstrap resumes.
pub fn configured_release_roles(
    config_path: &Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    configured_release_roles_from_source(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()).into())
}

// Enumerate the local install targets: root plus the ordinary roles owned by its subnet.
pub fn configured_install_targets(
    config_path: &Path,
    root_canister: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut targets = vec![root_canister.to_string()];
    targets.extend(configured_release_roles(config_path)?);
    Ok(targets)
}

// Enumerate the configured ordinary roles for the single subnet that owns `root`.
fn configured_release_roles_from_source(
    config_source: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let mut roles = BTreeSet::new();
    let mut root_subnet_roles = None;

    for (subnet_role, subnet) in &config.subnets {
        if !subnet
            .canisters
            .keys()
            .any(canic::ids::CanisterRole::is_root)
        {
            continue;
        }

        if root_subnet_roles.is_some() {
            return Err(format!(
                "multiple subnets define a root canister; release-set staging requires exactly one root subnet (found at least '{subnet_role}')"
            )
            .into());
        }

        root_subnet_roles = Some(
            subnet
                .canisters
                .keys()
                .filter(|role| !role.is_root() && !role.is_wasm_store())
                .map(|role| role.as_str().to_string())
                .collect::<Vec<_>>(),
        );
    }

    let root_subnet_roles = root_subnet_roles.ok_or_else(|| {
        "no subnet defines a root canister; release-set staging requires exactly one root subnet"
            .to_string()
    })?;

    for role in root_subnet_roles {
        roles.insert(role);
    }

    Ok(roles.into_iter().collect())
}

// Read the reference root canister version so staged release versions match the install.
pub fn load_root_package_version(
    root_manifest_path: &Path,
    workspace_manifest_path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let manifest_source = fs::read_to_string(root_manifest_path)?;
    let manifest = toml::from_str::<TomlValue>(&manifest_source)?;
    let version_value = manifest
        .get("package")
        .and_then(TomlValue::as_table)
        .and_then(|package| package.get("version"))
        .ok_or_else(|| {
            format!(
                "missing package.version in {}",
                root_manifest_path.display()
            )
        })?;

    if let Some(version) = version_value.as_str() {
        return Ok(version.to_string());
    }

    if version_value
        .as_table()
        .and_then(|value| value.get("workspace"))
        .and_then(TomlValue::as_bool)
        == Some(true)
    {
        return load_workspace_package_version(workspace_manifest_path);
    }

    Err(format!(
        "unsupported package.version format in {}",
        root_manifest_path.display()
    )
    .into())
}

// Resolve the shared workspace package version used by reference canisters.
pub fn load_workspace_package_version(
    workspace_manifest_path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let manifest_source = fs::read_to_string(workspace_manifest_path)?;
    let manifest = toml::from_str::<TomlValue>(&manifest_source)?;
    let version = manifest
        .get("workspace")
        .and_then(TomlValue::as_table)
        .and_then(|workspace| workspace.get("package"))
        .and_then(TomlValue::as_table)
        .and_then(|package| package.get("version"))
        .and_then(TomlValue::as_str)
        .ok_or_else(|| {
            format!(
                "missing workspace.package.version in {}",
                workspace_manifest_path.display()
            )
        })?;

    Ok(version.to_string())
}

// Read the current host wall clock so staged manifests use a stable whole-second
// timestamp without depending on an exported root time endpoint.
pub fn root_time_secs(root_canister: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let _ = root_canister;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("system clock before unix epoch: {err}"))?;
    Ok(now.as_secs())
}

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
fn build_release_set_entry(
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
fn read_release_artifact(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
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

fn discover_workspace_root_from(path: &Path) -> Option<PathBuf> {
    let start = if path.is_file() { path.parent()? } else { path };

    for candidate in start.ancestors() {
        let manifest_path = candidate.join(WORKSPACE_MANIFEST_RELATIVE);
        if !manifest_path.is_file() {
            continue;
        }

        let manifest = fs::read_to_string(&manifest_path).ok()?;
        if manifest.contains("[workspace]") {
            return candidate.canonicalize().ok();
        }
    }

    None
}

fn discover_dfx_root_from(path: &Path) -> Option<PathBuf> {
    let start = if path.is_file() { path.parent()? } else { path };

    for candidate in start.ancestors() {
        let dfx_config = candidate.join(DFX_CONFIG_FILE);
        if dfx_config.is_file() {
            return candidate.canonicalize().ok();
        }
    }

    None
}

fn normalize_workspace_path(workspace_root: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        workspace_root.join(path)
    }
}

fn discover_canister_manifest_from_metadata(
    workspace_root: &Path,
    role_name: &str,
) -> Option<PathBuf> {
    let metadata = cargo_metadata(workspace_root).ok()?;
    let expected_package_name = format!("canister_{role_name}");

    metadata
        .packages
        .into_iter()
        .find(|package| {
            package_declares_role(package, role_name) || package.name == expected_package_name
        })
        .map(|package| package.manifest_path)
}

fn package_declares_role(package: &CargoMetadataPackage, role_name: &str) -> bool {
    package
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get("canic"))
        .and_then(|canic| canic.get("role"))
        .and_then(JsonValue::as_str)
        == Some(role_name)
}

fn cargo_metadata(workspace_root: &Path) -> Result<CargoMetadata, Box<dyn std::error::Error>> {
    let output = Command::new("cargo")
        .current_dir(workspace_root)
        .args([
            "metadata",
            "--format-version=1",
            "--no-deps",
            "--manifest-path",
            &workspace_root.join("Cargo.toml").display().to_string(),
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(serde_json::from_slice(&output.stdout)?)
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

#[cfg(test)]
mod tests {
    use super::{
        canister_manifest_path, canisters_root, config_path, configured_install_targets,
        configured_release_roles_from_source, read_release_artifact, root_manifest_path,
    };
    use flate2::{Compression, write::GzEncoder};
    use std::{
        fs,
        io::Write,
        path::Path,
        sync::{Mutex, OnceLock},
    };

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    const REAL_CONFIG: &str = r#"
controllers = []
app_directory = ["user_hub", "scale_hub"]

[app]
init_mode = "enabled"
[app.whitelist]

[auth.delegated_tokens]
enabled = true
ecdsa_key_name = "test_key_1"

[standards]
icrc21 = true

[subnets.prime]
auto_create = ["app", "user_hub", "scale_hub"]
subnet_directory = ["app", "user_hub", "scale_hub"]
pool.minimum_size = 3

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"

[subnets.prime.canisters.user_hub]
kind = "singleton"

[subnets.prime.canisters.user_hub.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.max_shards = 4

[subnets.prime.canisters.user_shard]
kind = "shard"

[subnets.prime.canisters.user_shard.delegated_auth]
signer = true
verifier = true

[subnets.prime.canisters.minimal]
kind = "replica"

[subnets.prime.canisters.scale_hub]
kind = "singleton"
topup_policy.threshold = "10T"
topup_policy.amount = "4T"

[subnets.prime.canisters.scale_hub.scaling.pools.scales]
canister_role = "scale"
policy.min_workers = 2

[subnets.prime.canisters.scale]
kind = "replica"

[subnets.general]

[subnets.general.canisters.minimal]
kind = "replica"
"#;

    #[test]
    fn configured_release_roles_only_uses_root_subnet() {
        let roles = configured_release_roles_from_source(REAL_CONFIG).unwrap();
        assert_eq!(
            roles,
            vec![
                "app".to_string(),
                "minimal".to_string(),
                "scale".to_string(),
                "scale_hub".to_string(),
                "user_hub".to_string(),
                "user_shard".to_string(),
            ]
        );
    }

    #[test]
    fn configured_release_roles_rejects_multiple_root_subnets() {
        let config = format!(
            "{REAL_CONFIG}\n[subnets.backup]\n[subnets.backup.canisters.root]\nkind = \"root\"\n"
        );

        assert!(configured_release_roles_from_source(&config).is_err());
    }

    #[test]
    fn configured_release_roles_rejects_missing_root_subnet() {
        let config = REAL_CONFIG.replace("[subnets.prime.canisters.root]\nkind = \"root\"\n\n", "");

        assert!(configured_release_roles_from_source(&config).is_err());
    }

    #[test]
    fn configured_install_targets_include_root_plus_release_roles() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join(format!(
            "canic-installer-install-targets-{}-{}.toml",
            std::process::id(),
            1
        ));
        fs::write(&config_path, REAL_CONFIG).unwrap();

        let targets = configured_install_targets(&config_path, "root").unwrap();
        assert_eq!(
            targets,
            vec![
                "root".to_string(),
                "app".to_string(),
                "minimal".to_string(),
                "scale".to_string(),
                "scale_hub".to_string(),
                "user_hub".to_string(),
                "user_shard".to_string(),
            ]
        );

        let _ = fs::remove_file(config_path);
    }

    #[test]
    fn read_release_artifact_accepts_gzipped_wasm() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!(
            "canic-installer-valid-artifact-{}-{}.wasm.gz",
            std::process::id(),
            1
        ));
        let bytes = gzipped_bytes(b"\0asm\x01\0\0\0payload");
        fs::write(&path, &bytes).unwrap();

        let read_back = read_release_artifact(&path).unwrap();
        assert_eq!(read_back, bytes);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_release_artifact_rejects_non_gzip_bytes() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!(
            "canic-installer-invalid-artifact-{}-{}.wasm.gz",
            std::process::id(),
            2
        ));
        fs::write(&path, b"not gzip").unwrap();

        let err = read_release_artifact(&path).unwrap_err();
        assert!(err.to_string().contains("not gzip-compressed"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_release_artifact_rejects_non_wasm_gzip() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!(
            "canic-installer-invalid-artifact-{}-{}.wasm.gz",
            std::process::id(),
            3
        ));
        fs::write(&path, gzipped_bytes(b"hello world")).unwrap();

        let err = read_release_artifact(&path).unwrap_err();
        assert!(
            err.to_string()
                .contains("does not decompress to a wasm module")
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn canisters_root_defaults_to_workspace_canisters() {
        let _env = env_lock();
        clear_path_env();

        assert_eq!(
            canisters_root(Path::new("/tmp/workspace")),
            Path::new("/tmp/workspace/canisters")
        );
    }

    #[test]
    fn canisters_root_respects_explicit_override() {
        let _env = env_lock();
        clear_path_env();
        unsafe {
            std::env::set_var("CANIC_CANISTERS_ROOT", "src/canisters");
        }

        assert_eq!(
            canisters_root(Path::new("/tmp/workspace")),
            Path::new("/tmp/workspace/src/canisters")
        );
    }

    #[test]
    fn canisters_root_infers_parent_from_config_path() {
        let _env = env_lock();
        clear_path_env();
        unsafe {
            std::env::set_var("CANIC_CONFIG_PATH", "src/canisters/canic.toml");
        }

        assert_eq!(
            canisters_root(Path::new("/tmp/workspace")),
            Path::new("/tmp/workspace/src/canisters")
        );
        assert_eq!(
            config_path(Path::new("/tmp/workspace")),
            Path::new("/tmp/workspace/src/canisters/canic.toml")
        );
        assert_eq!(
            root_manifest_path(Path::new("/tmp/workspace")),
            Path::new("/tmp/workspace/src/canisters/root/Cargo.toml")
        );
    }

    #[test]
    fn manifest_discovery_uses_workspace_metadata_for_nested_canisters() {
        let _env = env_lock();
        clear_path_env();
        let temp_root = temp_test_dir("metadata-nested");
        fs::create_dir_all(temp_root.join("src/canisters/project/ledger/src")).unwrap();
        fs::create_dir_all(temp_root.join("src/canisters/root/src")).unwrap();
        fs::write(
            temp_root.join("Cargo.toml"),
            r#"[workspace]
members = [
    "src/canisters/project/ledger",
    "src/canisters/root",
]
resolver = "2"
"#,
        )
        .unwrap();
        fs::write(
            temp_root.join("src/canisters/project/ledger/Cargo.toml"),
            r#"[package]
name = "canister_project_ledger"
version = "0.1.0"
edition = "2024"
"#,
        )
        .unwrap();
        fs::write(
            temp_root.join("src/canisters/project/ledger/src/lib.rs"),
            "",
        )
        .unwrap();
        fs::write(
            temp_root.join("src/canisters/root/Cargo.toml"),
            r#"[package]
name = "canister_root"
version = "0.1.0"
edition = "2024"
"#,
        )
        .unwrap();
        fs::write(temp_root.join("src/canisters/root/src/lib.rs"), "").unwrap();

        assert_eq!(
            canister_manifest_path(&temp_root, "project_ledger"),
            temp_root.join("src/canisters/project/ledger/Cargo.toml")
        );
        assert_eq!(canisters_root(&temp_root), temp_root.join("src/canisters"));
        assert_eq!(
            root_manifest_path(&temp_root),
            temp_root.join("src/canisters/root/Cargo.toml")
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn manifest_discovery_respects_explicit_metadata_role() {
        let _env = env_lock();
        clear_path_env();
        let temp_root = temp_test_dir("metadata-role");
        fs::create_dir_all(temp_root.join("src/canisters/project/ledger/src")).unwrap();
        fs::create_dir_all(temp_root.join("src/canisters/root/src")).unwrap();
        fs::write(
            temp_root.join("Cargo.toml"),
            r#"[workspace]
members = [
    "src/canisters/project/ledger",
    "src/canisters/root",
]
resolver = "2"
"#,
        )
        .unwrap();
        fs::write(
            temp_root.join("src/canisters/project/ledger/Cargo.toml"),
            r#"[package]
name = "toko_project_ledger"
version = "0.1.0"
edition = "2024"

[package.metadata.canic]
role = "project_ledger"
"#,
        )
        .unwrap();
        fs::write(
            temp_root.join("src/canisters/project/ledger/src/lib.rs"),
            "",
        )
        .unwrap();
        fs::write(
            temp_root.join("src/canisters/root/Cargo.toml"),
            r#"[package]
name = "canister_root"
version = "0.1.0"
edition = "2024"
"#,
        )
        .unwrap();
        fs::write(temp_root.join("src/canisters/root/src/lib.rs"), "").unwrap();

        assert_eq!(
            canister_manifest_path(&temp_root, "project_ledger"),
            temp_root.join("src/canisters/project/ledger/Cargo.toml")
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    fn gzipped_bytes(input: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(input).unwrap();
        encoder.finish().unwrap()
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock must not be poisoned")
    }

    fn clear_path_env() {
        for key in [
            "CANIC_CANISTERS_ROOT",
            "CANIC_CONFIG_PATH",
            "CANIC_ROOT_MANIFEST_PATH",
            "CANIC_WORKSPACE_MANIFEST_PATH",
        ] {
            unsafe {
                std::env::remove_var(key);
            }
        }
    }

    fn temp_test_dir(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "canic-installer-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time must be after unix epoch")
                .as_nanos()
        ))
    }
}
