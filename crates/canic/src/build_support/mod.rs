use canic_core::bootstrap::compiled::ConfigModel;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::{
    env,
    fmt::Write as _,
    fs,
    io::Read as _,
    path::{Path, PathBuf},
};

const ROOT_RELEASE_ASSET_DIR: &str = "embedded_root_release_bundle";
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

    if !artifact_path.is_file() {
        assert!(
            !strict_artifacts,
            "root bootstrap requires the build-produced wasm_store artifact at {}; build wasm_store through the normal DFX/custom build path first",
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

#[must_use]
pub fn emit_root_release_bundle(config_path: &Path, config: &ConfigModel) -> bool {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set for root build"),
    );
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set for root build"));
    let workspace_root = discover_workspace_root(&manifest_dir);
    let artifact_root = discover_release_artifact_root(&workspace_root);
    let asset_dir = out_dir.join(ROOT_RELEASE_ASSET_DIR);
    let strict_artifacts =
        env::var("CANIC_REQUIRE_EMBEDDED_RELEASE_ARTIFACTS").is_ok_and(|value| value == "1");

    fs::create_dir_all(&asset_dir).expect("create embedded root release asset dir");

    println!("cargo:rerun-if-changed={}", workspace_root.display());
    println!("cargo:rerun-if-changed={}", config_path.display());
    println!("cargo:rerun-if-changed={}", artifact_root.display());
    println!("cargo:rerun-if-env-changed=CANIC_REQUIRE_EMBEDDED_RELEASE_ARTIFACTS");

    let mut built_entries = Vec::new();

    for role_name in configured_release_roles(config) {
        let artifact_path = resolve_release_wasm_path(&artifact_root, &role_name);
        if !artifact_path.is_file() {
            assert!(
                !strict_artifacts,
                "root release bundle requires the build-produced gzip artifact for role '{role_name}' at {}; build the child canisters through the normal DFX/custom build path first",
                artifact_root
                    .join(&role_name)
                    .join(format!("{role_name}.wasm.gz"))
                    .display()
            );
            println!(
                "cargo:warning=canic root bundle skipped role '{role_name}': missing built artifact at {}; build dependencies first if this role should bootstrap automatically",
                artifact_path.display()
            );
            continue;
        }

        let out_wasm = asset_dir.join(format!("{role_name}.wasm.gz"));
        fs::copy(&artifact_path, &out_wasm).unwrap_or_else(|err| {
            panic!(
                "copy embedded release wasm for role '{role_name}' from {} to {} failed: {err}",
                artifact_path.display(),
                out_wasm.display()
            )
        });
        built_entries.push((role_name, out_wasm));
    }

    let generated = render_root_release_bundle_source(&built_entries);
    let generated_path = out_dir.join("canic.root-release-bundle.rs");
    fs::write(&generated_path, generated).expect("write embedded root release bundle source");

    let generated_abs = generated_path
        .canonicalize()
        .expect("canonicalize embedded root release bundle source path");
    println!(
        "cargo:rustc-env=CANIC_ROOT_RELEASE_BUNDLE_PATH={}",
        generated_abs.display()
    );
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
    let network = env::var("DFX_NETWORK").unwrap_or_else(|_| "local".to_string());
    let network_root = workspace_root.join(".dfx").join(&network).join("canisters");
    if network_root.is_dir() {
        return network_root;
    }

    let local_root = workspace_root.join(".dfx").join("local").join("canisters");
    if local_root.is_dir() {
        return local_root;
    }

    network_root
}

fn configured_release_roles(config: &ConfigModel) -> Vec<String> {
    let mut roles = Vec::new();

    for subnet in config.subnets.values() {
        for role in subnet.canisters.keys() {
            if role.is_root() || role.is_wasm_store() {
                continue;
            }

            let role_name = role.as_str().to_string();
            if !roles.iter().any(|existing| existing == &role_name) {
                roles.push(role_name);
            }
        }
    }

    roles.sort();
    roles
}

fn resolve_release_wasm_path(artifact_root: &Path, role_name: &str) -> PathBuf {
    artifact_root
        .join(role_name)
        .join(format!("{role_name}.wasm.gz"))
}

fn render_root_release_bundle_source(entries: &[(String, PathBuf)]) -> String {
    let mut rendered = String::from("&[\n");

    for (role_name, wasm_path) in entries {
        let path = wasm_path.to_string_lossy();
        rendered.push_str("    canic::__internal::core::bootstrap::EmbeddedRootReleaseEntry {\n");
        let _ = writeln!(rendered, "        role: {role_name:?},");
        let _ = writeln!(rendered, "        wasm_module: include_bytes!({path:?}),");
        rendered.push_str("    },\n");
    }

    rendered.push(']');
    rendered.push('\n');
    rendered
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
