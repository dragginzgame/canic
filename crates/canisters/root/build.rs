use sha2::{Digest, Sha256};
use std::{env, fmt::Write as _, fs, path::PathBuf};

const RELEASE_ROLES: &[&str] = &[
    "app",
    "user_hub",
    "user_shard",
    "minimal",
    "scale_hub",
    "scale",
    "shard_hub",
    "shard",
    "test",
];

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

// Generate a compact manifest-only WasmStore release catalog for root bootstrap.
fn write_embedded_wasm_store_release_catalog() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .ancestors()
        .nth(3)
        .expect("workspace root must exist");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let version = env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION");

    let mut body = String::from(
        "#[must_use]\npub fn embedded_wasm_store_release_catalog() -> Vec<canic::dto::template::WasmStoreCatalogEntryResponse> {\n    vec![\n",
    );

    for role in RELEASE_ROLES {
        let wasm_path = repo_root
            .join(".dfx/local/canisters")
            .join(role)
            .join(format!("{role}.wasm.gz"));
        println!("cargo:rerun-if-changed={}", wasm_path.display());

        let bytes = fs::read(&wasm_path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", wasm_path.display()));
        let payload_size_bytes = bytes.len();
        let payload_hash = Sha256::digest(&bytes);
        let hash_bytes = payload_hash
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        let role_const = role.to_ascii_uppercase();
        let payload_size_literal = format_u64_literal(payload_size_bytes);

        let _ = writeln!(
            body,
            "        canic::dto::template::WasmStoreCatalogEntryResponse {{ role: canic_internal::canister::{role_const}, template_id: canic::ids::TemplateId::new(\"embedded:{role}\"), version: canic::ids::TemplateVersion::new(\"{version}\"), payload_hash: vec![{hash_bytes}], payload_size_bytes: {payload_size_literal} }},"
        );
    }

    body.push_str("    ]\n}\n");

    fs::write(out_dir.join("embedded_store_release_catalog.rs"), body)
        .expect("write embedded WasmStore release catalog");
}

fn main() {
    canic::build_root!("../canic.toml");
    write_embedded_wasm_store_release_catalog();
}
