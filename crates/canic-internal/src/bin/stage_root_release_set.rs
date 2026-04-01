use canic::protocol;
use canic_core::bootstrap::parse_config_model;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    env,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const ROOT_RELEASE_CHUNK_BYTES: usize = 1024 * 1024;
const ROOT_CONFIG_RELATIVE: &str = "canisters/canic.toml";
const ROOT_MANIFEST_RELATIVE: &str = "canisters/root/Cargo.toml";
const WORKSPACE_MANIFEST_RELATIVE: &str = "Cargo.toml";

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

// Stage the configured ordinary release set into root, then resume bootstrap.
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let root_canister = env::args()
        .nth(1)
        .or_else(|| env::var("ROOT_CANISTER").ok())
        .unwrap_or_else(|| "root".to_string());
    let workspace_root = workspace_root()?;
    let network = env::var("DFX_NETWORK").unwrap_or_else(|_| "local".to_string());
    let artifact_root = resolve_artifact_root(&workspace_root, &network)?;
    let release_version = load_root_package_version(
        &workspace_root.join(ROOT_MANIFEST_RELATIVE),
        &workspace_root.join(WORKSPACE_MANIFEST_RELATIVE),
    )?;
    let now_secs = root_time_secs(&root_canister)?;

    for role_name in configured_release_roles(&workspace_root.join(ROOT_CONFIG_RELATIVE))? {
        stage_release_role(
            &root_canister,
            &artifact_root,
            &role_name,
            &release_version,
            now_secs,
        )?;
    }

    let _ = dfx_call(
        &root_canister,
        protocol::CANIC_WASM_STORE_BOOTSTRAP_RESUME_ROOT_ADMIN,
        None,
        None,
    )?;
    Ok(())
}

// Resolve the workspace root from the internal helper crate location.
fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    Ok(workspace_root)
}

// Prefer the selected DFX network artifact root and fall back to local when present.
fn resolve_artifact_root(
    workspace_root: &Path,
    network: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let preferred = workspace_root.join(".dfx").join(network).join("canisters");
    if preferred.is_dir() {
        return Ok(preferred);
    }

    let fallback = workspace_root.join(".dfx/local/canisters");
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

// Read the reference root canister version so staged release versions match the install.
fn load_root_package_version(
    root_manifest_path: &Path,
    workspace_manifest_path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let manifest_source = fs::read_to_string(root_manifest_path)?;
    let manifest = toml::from_str::<Value>(&manifest_source)?;
    let version_value = manifest
        .get("package")
        .and_then(Value::as_object)
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
        .as_object()
        .and_then(|value| value.get("workspace"))
        .and_then(Value::as_bool)
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
fn load_workspace_package_version(
    workspace_manifest_path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let manifest_source = fs::read_to_string(workspace_manifest_path)?;
    let manifest = toml::from_str::<Value>(&manifest_source)?;
    let version = manifest
        .get("workspace")
        .and_then(Value::as_object)
        .and_then(|workspace| workspace.get("package"))
        .and_then(Value::as_object)
        .and_then(|package| package.get("version"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!(
                "missing workspace.package.version in {}",
                workspace_manifest_path.display()
            )
        })?;

    Ok(version.to_string())
}

// Enumerate the configured ordinary roles that root must publish before bootstrap resumes.
fn configured_release_roles(config_path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config_source = fs::read_to_string(config_path)?;
    let config = parse_config_model(&config_source)
        .map_err(|err| format!("invalid {}: {err}", config_path.display()))?;
    let mut roles = BTreeSet::new();

    for subnet in config.subnets.values() {
        for role in subnet.canisters.keys() {
            if role.is_root() || role.is_wasm_store() {
                continue;
            }

            roles.insert(role.as_str().to_string());
        }
    }

    Ok(roles.into_iter().collect())
}

// Read the current root time so staged manifests use replica timestamps.
fn root_time_secs(root_canister: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let payload = dfx_call(root_canister, protocol::CANIC_TIME, None, Some("json"))?;
    let data = serde_json::from_str::<Value>(&payload)?;
    let now_nanos = data
        .get("Ok")
        .and_then(json_u64)
        .ok_or_else(|| format!("unexpected canic_time response: {payload}"))?;

    Ok(now_nanos / 1_000_000_000)
}

// Stage one configured role manifest, prepare its chunk set, and upload all bytes to root.
fn stage_release_role(
    root_canister: &str,
    artifact_root: &Path,
    role_name: &str,
    release_version: &str,
    now_secs: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let artifact_path = artifact_root
        .join(role_name)
        .join(format!("{role_name}.wasm.gz"));
    let wasm_module = fs::read(&artifact_path)?;
    if wasm_module.is_empty() {
        return Err(format!("release artifact is empty: {}", artifact_path.display()).into());
    }

    let template_id = format!("embedded:{role_name}");
    let payload_hash = wasm_hash(&wasm_module);
    let chunks = wasm_module
        .chunks(ROOT_RELEASE_CHUNK_BYTES)
        .map(<[u8]>::to_vec)
        .collect::<Vec<_>>();

    let manifest = format!(
        "(record {{ template_id = {}; role = {}; version = {}; payload_hash = {}; \
         payload_size_bytes = {} : nat64; store_binding = \"bootstrap\"; \
         chunking_mode = variant {{ Chunked }}; manifest_state = variant {{ Approved }}; \
         approved_at = opt ({} : nat64); created_at = {} : nat64 }})",
        idl_text(&template_id),
        idl_text(role_name),
        idl_text(release_version),
        idl_blob(&payload_hash),
        wasm_module.len(),
        now_secs,
        now_secs,
    );
    let _ = dfx_call(
        root_canister,
        protocol::CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
        Some(&manifest),
        None,
    )?;

    let chunk_hashes = chunks
        .iter()
        .map(|chunk| idl_blob(&wasm_hash(chunk)))
        .collect::<Vec<_>>()
        .join("; ");
    let prepare = format!(
        "(record {{ template_id = {}; version = {}; payload_hash = {}; \
         payload_size_bytes = {} : nat64; chunk_hashes = vec {{ {} }} }})",
        idl_text(&template_id),
        idl_text(release_version),
        idl_blob(&payload_hash),
        wasm_module.len(),
        chunk_hashes,
    );
    let _ = dfx_call(
        root_canister,
        protocol::CANIC_TEMPLATE_PREPARE_ADMIN,
        Some(&prepare),
        None,
    )?;

    for (chunk_index, chunk) in chunks.iter().enumerate() {
        let request = format!(
            "(record {{ template_id = {}; version = {}; chunk_index = {} : nat32; bytes = {} }})",
            idl_text(&template_id),
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
    }

    Ok(())
}

// Run one `dfx canister call` and return stdout, preserving stderr on failure.
fn dfx_call(
    canister: &str,
    method: &str,
    argument: Option<&str>,
    output: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut command = Command::new("dfx");
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

// Persist one temporary Candid argument file for `dfx --argument-file`.
fn write_argument_file(argument: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = env::temp_dir().join(format!(
        "canic-stage-root-release-set-{}-{unique}.did",
        std::process::id()
    ));
    fs::write(&path, argument)?;
    Ok(path)
}

// Encode one string as a Candid text literal.
fn idl_text(value: &str) -> String {
    serde_json::to_string(value).expect("string literal encoding must succeed")
}

// Encode one blob as a Candid text blob literal.
fn idl_blob(bytes: &[u8]) -> String {
    let mut encoded = String::from("blob \"");

    for byte in bytes {
        let _ = write!(encoded, "\\{byte:02X}");
    }

    encoded.push('"');
    encoded
}

// Compute the canonical SHA-256 hash used by the template staging APIs.
fn wasm_hash(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().to_vec()
}

// Decode a JSON nat that may be emitted as either a number or a string.
fn json_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|raw| raw.parse::<u64>().ok()))
}
