use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

macro_rules! collect_release_roles {
    ($cfg:expr) => {{
        let mut roles = BTreeSet::new();

        for subnet in $cfg.subnets.values() {
            for (role, canister_cfg) in &subnet.canisters {
                if canister_cfg.kind.to_string() == "root" {
                    continue;
                }

                roles.insert(role.as_str().to_string());
            }
        }

        roles.into_iter().collect::<Vec<_>>()
    }};
}

// Format one usize as a grouped Rust `u64` literal for generated source.
fn format_u64_literal(value: usize) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 4);

    for (idx, ch) in digits.chars().rev().enumerate() {
        if idx != 0 && idx % 3 == 0 {
            out.push('_');
        }
        out.push(ch);
    }

    out = out.chars().rev().collect::<String>();
    out.push_str("_u64");
    out
}

// Resolve the workspace root so generated code can address built `.dfx` artifacts.
fn workspace_root(manifest_dir: &Path) -> PathBuf {
    manifest_dir
        .ancestors()
        .find(|dir| {
            let cargo_toml = dir.join("Cargo.toml");
            cargo_toml.is_file()
                && fs::read_to_string(&cargo_toml)
                    .is_ok_and(|contents| contents.contains("[workspace]"))
        })
        .map(Path::to_path_buf)
        .expect("workspace root must contain Cargo.toml with [workspace]")
}

// Select the built artifact namespace used for generated template release input.
fn dfx_network_dir() -> &'static str {
    match env::var("DFX_NETWORK") {
        Ok(value) if value == "ic" => "ic",
        Ok(value) if value == "local" => "local",
        Ok(value) => panic!("unsupported DFX_NETWORK '{value}'; expected 'local' or 'ic'"),
        Err(_) => "local",
    }
}

// Keep strict artifact enforcement only for the real canister bundle build path.
fn require_release_artifacts() -> bool {
    env::var_os("CANIC_REQUIRE_EMBEDDED_RELEASE_ARTIFACTS").is_some()
}

// Resolve one built wasm artifact path for a config role or bootstrap canister.
fn built_wasm_path(manifest_dir: &Path, role: &str) -> PathBuf {
    workspace_root(manifest_dir)
        .join(".dfx")
        .join(dfx_network_dir())
        .join("canisters")
        .join(role)
        .join(format!("{role}.wasm.gz"))
}

// Generate a compact manifest-only WasmStore release catalog for root bootstrap.
fn write_embedded_wasm_store_release_catalog(
    roles: &[String],
    manifest_dir: &Path,
    out_dir: &Path,
    version: &str,
) {
    let mut body = String::from(
        "#[must_use]\npub fn embedded_wasm_store_release_catalog() -> Vec<canic::dto::template::WasmStoreCatalogEntryResponse> {\n    vec![\n",
    );
    let require_artifacts = require_release_artifacts();

    for role in roles {
        let wasm_path = built_wasm_path(manifest_dir, role);
        println!("cargo:rerun-if-changed={}", wasm_path.display());

        let bytes = match fs::read(&wasm_path) {
            Ok(bytes) => bytes,
            Err(err) if !require_artifacts => {
                println!(
                    "cargo:warning=skipping release catalog entry for role '{role}'; artifact missing at {}: {err}",
                    wasm_path.display()
                );
                continue;
            }
            Err(err) => {
                panic!(
                    "failed to read configured release artifact for role '{role}' at {}: {err}",
                    wasm_path.display()
                )
            }
        };
        let payload_size_bytes = bytes.len();
        let payload_hash = Sha256::digest(&bytes);
        let hash_bytes = payload_hash
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        let payload_size_literal = format_u64_literal(payload_size_bytes);

        let _ = writeln!(
            body,
            "        canic::dto::template::WasmStoreCatalogEntryResponse {{ role: canic::ids::CanisterRole::new(\"{role}\"), template_id: canic::ids::TemplateId::new(\"embedded:{role}\"), version: canic::ids::TemplateVersion::new(\"{version}\"), payload_hash: vec![{hash_bytes}], payload_size_bytes: {payload_size_literal} }},"
        );
    }

    body.push_str("    ]\n}\n");

    fs::write(out_dir.join("embedded_store_release_catalog.rs"), body)
        .expect("write embedded WasmStore release catalog");
}

// Generate the root-local inline bootstrap payload for the first `wasm_store`.
fn write_embedded_wasm_store_bootstrap_release_set(manifest_dir: &Path, out_dir: &Path) {
    let wasm_path = built_wasm_path(manifest_dir, "wasm_store");
    println!("cargo:rerun-if-changed={}", wasm_path.display());

    let body = if wasm_path.is_file() {
        format!(
            "pub static EMBEDDED_WASM_STORE_BOOTSTRAP_RELEASE_SET: &[(canic::ids::CanisterRole, &[u8])] = &[\n    (canic::ids::CanisterRole::new(\"wasm_store\"), include_bytes!(r#\"{}\"#) as &[u8]),\n];\n",
            wasm_path.display()
        )
    } else if require_release_artifacts() {
        panic!(
            "bootstrap wasm_store artifact is missing at {}",
            wasm_path.display()
        );
    } else {
        println!(
            "cargo:warning=skipping embedded wasm_store bootstrap payload; artifact missing at {}",
            wasm_path.display()
        );
        "pub static EMBEDDED_WASM_STORE_BOOTSTRAP_RELEASE_SET: &[(canic::ids::CanisterRole, &[u8])] = &[];\n"
            .to_string()
    };

    fs::write(
        out_dir.join("embedded_wasm_store_bootstrap_release_set.rs"),
        body,
    )
    .expect("write embedded wasm_store bootstrap release set");
}

fn main() {
    canic::build_root_with!("../canic.toml", |_cfg_str, _cfg_path, cfg| {
        let manifest_dir =
            PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
        let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
        let version = env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION");
        let roles = collect_release_roles!(cfg);

        write_embedded_wasm_store_release_catalog(&roles, &manifest_dir, &out_dir, &version);
        write_embedded_wasm_store_bootstrap_release_set(&manifest_dir, &out_dir);
    });
}
