use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::{
    env,
    fmt::Write as _,
    fs,
    io::Read as _,
    path::{Path, PathBuf},
};

const ROOT_WASM_STORE_BOOTSTRAP_ROLE: &str = "wasm_store";
const ROOT_WASM_STORE_BOOTSTRAP_RELEASE_SET_FILE: &str =
    "canic.root-wasm-store-bootstrap-release-set.rs";
const ROOT_WASM_STORE_BOOTSTRAP_ASSET_FILE: &str = "canic.root-wasm-store-bootstrap.wasm.gz";

struct EmbeddedArtifactMetadata {
    artifact_kind: &'static str,
    artifact_size_bytes: u64,
    artifact_sha256_hex: String,
    decompressed_size_bytes: Option<u64>,
    decompressed_sha256_hex: Option<String>,
}

/// Read a Canic config source, or generate a minimal standalone config when allowed.
#[must_use]
pub fn read_config_source_or_default(
    config_path: &Path,
    explicit_config: bool,
    default_role: Option<&str>,
) -> (String, bool) {
    match fs::read_to_string(config_path) {
        Ok(source) => (source, false),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let role = default_role
                .unwrap_or_else(|| panic!("Missing Canic config at {}", config_path.display()));

            assert!(
                !explicit_config,
                "Missing explicit Canic config at {}",
                config_path.display()
            );

            (standalone_config_source(role), true)
        }
        Err(err) => panic!("Failed to read {}: {err}", config_path.display()),
    }
}

/// Render the minimal topology needed by a standalone non-root canister.
#[must_use]
pub fn standalone_config_source(role: &str) -> String {
    assert!(
        !role.is_empty() && role != "root",
        "standalone Canic config requires a non-root role"
    );

    let role_key = toml_basic_string(role);

    format!(
        r#"controllers = []
app_index = []

[app]
init_mode = "enabled"

[app.whitelist]

[subnets.prime]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.{role_key}]
kind = "singleton"
"#
    )
}

#[must_use]
pub fn emit_root_wasm_store_bootstrap_release_set(config_path: &Path) -> bool {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set for root build"),
    );
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set for root build"));
    let workspace_root = discover_workspace_root(&manifest_dir);
    let artifact_root = discover_release_artifact_root(&workspace_root);
    let generated_path = out_dir.join(ROOT_WASM_STORE_BOOTSTRAP_RELEASE_SET_FILE);
    let embedded_asset_path = out_dir.join(ROOT_WASM_STORE_BOOTSTRAP_ASSET_FILE);
    let strict_artifacts =
        env::var("CANIC_REQUIRE_EMBEDDED_RELEASE_ARTIFACTS").is_ok_and(|value| value == "1");
    let artifact_path = artifact_root
        .join(ROOT_WASM_STORE_BOOTSTRAP_ROLE)
        .join(format!("{ROOT_WASM_STORE_BOOTSTRAP_ROLE}.wasm.gz"));

    println!("cargo:rerun-if-changed={}", workspace_root.display());
    println!("cargo:rerun-if-changed={}", config_path.display());
    println!("cargo:rerun-if-changed={}", artifact_root.display());
    println!("cargo:rerun-if-changed={}", artifact_path.display());
    println!("cargo:rerun-if-env-changed=CANIC_REQUIRE_EMBEDDED_RELEASE_ARTIFACTS");
    println!("cargo:rerun-if-env-changed=CANIC_ICP_ROOT");

    if !artifact_path.is_file() {
        assert!(
            !strict_artifacts,
            "root bootstrap requires the build-produced wasm_store artifact at {}; build wasm_store through the normal Canic/ICP build path first",
            artifact_path.display()
        );

        println!(
            "cargo:warning=skipping embedded wasm_store bootstrap release set: missing build-produced artifact at {}",
            artifact_path.display()
        );
        return false;
    }

    let artifact_path = artifact_path
        .canonicalize()
        .expect("canonicalize build-produced wasm_store bootstrap artifact");
    fs::copy(&artifact_path, &embedded_asset_path).unwrap_or_else(|err| {
        panic!(
            "copy embedded wasm_store bootstrap artifact from {} to {} failed: {err}",
            artifact_path.display(),
            embedded_asset_path.display(),
        )
    });
    let embedded_asset_path = embedded_asset_path
        .canonicalize()
        .expect("canonicalize copied embedded wasm_store bootstrap artifact");
    let metadata = inspect_embedded_artifact(&artifact_path, &embedded_asset_path);
    println!(
        "cargo:warning=root bootstrap artifact: role=wasm_store source={} embedded={} kind={} size={} sha256={}",
        artifact_path.display(),
        embedded_asset_path.display(),
        metadata.artifact_kind,
        metadata.artifact_size_bytes,
        metadata.artifact_sha256_hex,
    );

    let generated = render_root_wasm_store_bootstrap_release_set_source(
        &artifact_path,
        &embedded_asset_path,
        &metadata,
    );
    fs::write(&generated_path, generated)
        .expect("write embedded wasm_store bootstrap release set source");

    let generated_abs = generated_path
        .canonicalize()
        .expect("canonicalize embedded wasm_store bootstrap release set source path");
    println!(
        "cargo:rustc-env=CANIC_ROOT_WASM_STORE_BOOTSTRAP_RELEASE_SET_PATH={}",
        generated_abs.display()
    );
    println!("cargo:rerun-if-changed={}", embedded_asset_path.display());
    println!("cargo:rerun-if-changed={}", generated_abs.display());
    true
}

fn discover_workspace_root(manifest_dir: &Path) -> PathBuf {
    for candidate in manifest_dir.ancestors() {
        let cargo_toml = candidate.join("Cargo.toml");
        if !cargo_toml.is_file() {
            continue;
        }

        let cargo_toml_text = fs::read_to_string(&cargo_toml)
            .unwrap_or_else(|err| panic!("read {} failed: {err}", cargo_toml.display()));

        if cargo_toml_text.contains("[workspace]") {
            return candidate.to_path_buf();
        }
    }

    panic!(
        "unable to discover workspace root from {}; expected an ancestor Cargo.toml with [workspace]",
        manifest_dir.display()
    );
}

fn discover_release_artifact_root(workspace_root: &Path) -> PathBuf {
    if let Ok(root) = env::var("CANIC_ICP_ROOT") {
        let icp_root = PathBuf::from(root);
        let network = env::var("ICP_ENVIRONMENT").unwrap_or_else(|_| "local".to_string());
        let network_root = icp_root.join(".icp").join(&network).join("canisters");
        if network_root.is_dir() {
            return network_root;
        }

        let local_root = icp_root.join(".icp").join("local").join("canisters");
        if local_root.is_dir() {
            return local_root;
        }

        return network_root;
    }

    let network = env::var("ICP_ENVIRONMENT").unwrap_or_else(|_| "local".to_string());
    let network_root = workspace_root.join(".icp").join(&network).join("canisters");
    if network_root.is_dir() {
        return network_root;
    }

    let local_root = workspace_root.join(".icp").join("local").join("canisters");
    if local_root.is_dir() {
        return local_root;
    }

    network_root
}

fn render_root_wasm_store_bootstrap_release_set_source(
    source_artifact_path: &Path,
    embedded_artifact_path: &Path,
    metadata: &EmbeddedArtifactMetadata,
) -> String {
    let source_path = source_artifact_path.to_string_lossy();
    let embedded_path = embedded_artifact_path.to_string_lossy();
    let mut rendered = String::from("&[\n");

    rendered.push_str("    canic::__internal::core::bootstrap::EmbeddedRootBootstrapEntry {\n");
    let _ = writeln!(
        rendered,
        "        role: {ROOT_WASM_STORE_BOOTSTRAP_ROLE:?},"
    );
    let _ = writeln!(
        rendered,
        "        wasm_module: include_bytes!({embedded_path:?}),"
    );
    let _ = writeln!(rendered, "        artifact_path: {source_path:?},");
    let _ = writeln!(
        rendered,
        "        embedded_artifact_path: {embedded_path:?},"
    );
    let _ = writeln!(
        rendered,
        "        artifact_kind: {:?},",
        metadata.artifact_kind
    );
    let _ = writeln!(
        rendered,
        "        artifact_size_bytes: {},",
        render_u64_literal(metadata.artifact_size_bytes)
    );
    let _ = writeln!(
        rendered,
        "        artifact_sha256_hex: {:?},",
        metadata.artifact_sha256_hex
    );
    match metadata.decompressed_size_bytes {
        Some(size) => {
            let _ = writeln!(
                rendered,
                "        decompressed_size_bytes: Some({}),",
                render_u64_literal(size)
            );
        }
        None => rendered.push_str("        decompressed_size_bytes: None,\n"),
    }
    match &metadata.decompressed_sha256_hex {
        Some(sha) => {
            let _ = writeln!(rendered, "        decompressed_sha256_hex: Some({sha:?}),");
        }
        None => rendered.push_str("        decompressed_sha256_hex: None,\n"),
    }
    rendered.push_str("    },\n");
    rendered.push(']');
    rendered.push('\n');
    rendered
}

fn inspect_embedded_artifact(
    source_artifact_path: &Path,
    embedded_artifact_path: &Path,
) -> EmbeddedArtifactMetadata {
    let bytes = fs::read(embedded_artifact_path).unwrap_or_else(|err| {
        panic!(
            "read embedded artifact metadata from {} failed: {err}",
            embedded_artifact_path.display()
        )
    });

    let artifact_kind = if is_gzip_payload(&bytes) {
        "gzip"
    } else if is_raw_wasm(&bytes) {
        "raw-wasm"
    } else {
        "opaque"
    };

    let decompressed = if artifact_kind == "gzip" {
        Some(decompress_gzip(&bytes, embedded_artifact_path))
    } else {
        None
    };

    validate_embedded_bootstrap_artifact(
        source_artifact_path,
        embedded_artifact_path,
        artifact_kind,
        &bytes,
        decompressed.as_deref(),
    );

    EmbeddedArtifactMetadata {
        artifact_kind,
        artifact_size_bytes: bytes.len() as u64,
        artifact_sha256_hex: sha256_hex(&bytes),
        decompressed_size_bytes: decompressed.as_ref().map(|decoded| decoded.len() as u64),
        decompressed_sha256_hex: decompressed.as_ref().map(|decoded| sha256_hex(decoded)),
    }
}

fn validate_embedded_bootstrap_artifact(
    source_artifact_path: &Path,
    embedded_artifact_path: &Path,
    artifact_kind: &str,
    artifact_bytes: &[u8],
    decompressed_bytes: Option<&[u8]>,
) {
    assert!(
        artifact_kind == "gzip",
        "root bootstrap requires the build-produced gzip artifact at {} (embedded copy {}), but found {} bytes with head {} and sha256={}",
        source_artifact_path.display(),
        embedded_artifact_path.display(),
        artifact_kind,
        hex_head(artifact_bytes),
        sha256_hex(artifact_bytes),
    );

    let decompressed_bytes = decompressed_bytes
        .expect("gzip artifact metadata must include decompressed bytes for validation");

    assert!(
        !decompressed_bytes.is_empty(),
        "root bootstrap artifact at {} (embedded copy {}) is invalid: gzip payload decompresses to zero bytes; compressed_size={} compressed_sha256={} compressed_head={}",
        source_artifact_path.display(),
        embedded_artifact_path.display(),
        artifact_bytes.len(),
        sha256_hex(artifact_bytes),
        hex_head(artifact_bytes),
    );

    assert!(
        is_raw_wasm(decompressed_bytes),
        "root bootstrap artifact at {} (embedded copy {}) is invalid: gzip payload does not decompress to a wasm module; decompressed_size={} decompressed_sha256={} decompressed_head={}",
        source_artifact_path.display(),
        embedded_artifact_path.display(),
        decompressed_bytes.len(),
        sha256_hex(decompressed_bytes),
        hex_head(decompressed_bytes),
    );
}

fn decompress_gzip(bytes: &[u8], artifact_path: &Path) -> Vec<u8> {
    let mut decoder = GzDecoder::new(bytes);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .unwrap_or_else(|err| {
            panic!(
                "decompress embedded gzip artifact {} failed: {err}",
                artifact_path.display()
            )
        });
    decompressed
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut rendered = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(rendered, "{byte:02x}");
    }
    rendered
}

fn render_u64_literal(value: u64) -> String {
    let digits = value.to_string();
    let mut rendered = String::with_capacity(digits.len() + digits.len() / 3);

    for (index, digit) in digits.chars().enumerate() {
        if index != 0 && (digits.len() - index).is_multiple_of(3) {
            rendered.push('_');
        }
        rendered.push(digit);
    }

    rendered
}

fn hex_head(bytes: &[u8]) -> String {
    let head = &bytes[..bytes.len().min(16)];
    let mut rendered = String::with_capacity(head.len() * 2);
    for byte in head {
        let _ = write!(rendered, "{byte:02x}");
    }
    rendered
}

fn is_gzip_payload(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b
}

fn is_raw_wasm(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0] == 0x00 && bytes[1] == 0x61 && bytes[2] == 0x73 && bytes[3] == 0x6d
}

// Escape a role name as a TOML basic string for quoted table keys.
fn toml_basic_string(value: &str) -> String {
    let mut rendered = String::with_capacity(value.len() + 2);
    rendered.push('"');

    for ch in value.chars() {
        match ch {
            '"' => rendered.push_str("\\\""),
            '\\' => rendered.push_str("\\\\"),
            '\u{08}' => rendered.push_str("\\b"),
            '\t' => rendered.push_str("\\t"),
            '\n' => rendered.push_str("\\n"),
            '\u{0c}' => rendered.push_str("\\f"),
            '\r' => rendered.push_str("\\r"),
            ch if ch.is_control() => {
                let _ = write!(rendered, "\\u{:04X}", ch as u32);
            }
            ch => rendered.push(ch),
        }
    }

    rendered.push('"');
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::bootstrap::parse_config_model;

    #[test]
    fn standalone_config_source_parses_for_plain_role() {
        let source = standalone_config_source("sandbox_minimal");
        let cfg = parse_config_model(&source).expect("generated standalone config parses");

        let prime = cfg.subnets.get("prime").expect("prime subnet exists");

        assert!(prime.canisters.contains_key("root"));
        assert!(prime.canisters.contains_key("sandbox_minimal"));
    }

    #[test]
    fn standalone_config_source_quotes_role_keys() {
        let source = standalone_config_source("demo.role");
        let cfg = parse_config_model(&source).expect("generated standalone config parses");

        let prime = cfg.subnets.get("prime").expect("prime subnet exists");

        assert!(prime.canisters.contains_key("demo.role"));
    }

    #[test]
    #[should_panic(expected = "standalone Canic config requires a non-root role")]
    fn standalone_config_source_rejects_root_role() {
        let _ = standalone_config_source("root");
    }

    #[test]
    fn read_config_source_or_default_generates_when_implicit_file_is_missing() {
        let missing_path =
            std::env::temp_dir().join(format!("canic-missing-default-{}.toml", std::process::id()));
        let (source, generated) =
            read_config_source_or_default(missing_path.as_path(), false, Some("test"));

        assert!(generated);
        assert!(source.contains("[subnets.prime.canisters.\"test\"]"));
    }
}
