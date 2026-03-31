use canic_core::bootstrap::compiled::ConfigModel;
use std::{
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

const ROOT_RELEASE_ASSET_DIR: &str = "embedded_root_release_bundle";
const ROOT_WASM_STORE_BOOTSTRAP_ROLE: &str = "wasm_store";
const ROOT_WASM_STORE_BOOTSTRAP_RELEASE_SET_FILE: &str =
    "canic.root-wasm-store-bootstrap-release-set.rs";

#[must_use]
pub fn emit_root_wasm_store_bootstrap_release_set(config_path: &Path) -> bool {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set for root build"),
    );
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set for root build"));
    let workspace_root = discover_workspace_root(&manifest_dir);
    let artifact_root = discover_release_artifact_root(&workspace_root);
    let generated_path = out_dir.join(ROOT_WASM_STORE_BOOTSTRAP_RELEASE_SET_FILE);
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
    println!(
        "cargo:warning=embedding build-produced wasm_store bootstrap artifact from {}",
        artifact_path.display()
    );

    let generated = render_root_wasm_store_bootstrap_release_set_source(&artifact_path);
    fs::write(&generated_path, generated)
        .expect("write embedded wasm_store bootstrap release set source");

    let generated_abs = generated_path
        .canonicalize()
        .expect("canonicalize embedded wasm_store bootstrap release set source path");
    println!(
        "cargo:rustc-env=CANIC_ROOT_WASM_STORE_BOOTSTRAP_RELEASE_SET_PATH={}",
        generated_abs.display()
    );
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

    fs::create_dir_all(&asset_dir).expect("create embedded root release asset dir");

    println!("cargo:rerun-if-changed={}", workspace_root.display());
    println!("cargo:rerun-if-changed={}", config_path.display());
    println!("cargo:rerun-if-changed={}", artifact_root.display());

    let mut built_entries = Vec::new();

    for role_name in configured_release_roles(config) {
        let artifact_path = resolve_release_wasm_path(&workspace_root, &artifact_root, &role_name);
        if !artifact_path.is_file() {
            println!(
                "cargo:warning=canic root bundle skipped role '{role_name}': missing built artifact at {}; build dependencies first if this role should bootstrap automatically",
                artifact_path.display()
            );
            continue;
        }

        let out_wasm = asset_dir.join(format!("{role_name}.wasm"));
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

fn discover_cargo_target_root(workspace_root: &Path) -> PathBuf {
    env::var_os("CARGO_TARGET_DIR").map_or_else(|| workspace_root.join("target"), PathBuf::from)
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

fn resolve_release_wasm_path(
    workspace_root: &Path,
    artifact_root: &Path,
    role_name: &str,
) -> PathBuf {
    let dfx_path = artifact_root
        .join(role_name)
        .join(format!("{role_name}.wasm"));
    if dfx_path.is_file() {
        return dfx_path;
    }

    discover_cargo_target_root(workspace_root)
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(format!("canister_{role_name}.wasm"))
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

fn render_root_wasm_store_bootstrap_release_set_source(artifact_path: &Path) -> String {
    let path = artifact_path.to_string_lossy();
    let mut rendered = String::from("&[\n");

    rendered.push_str("    canic::__internal::core::bootstrap::EmbeddedRootReleaseEntry {\n");
    let _ = writeln!(
        rendered,
        "        role: {ROOT_WASM_STORE_BOOTSTRAP_ROLE:?},"
    );
    let _ = writeln!(rendered, "        wasm_module: include_bytes!({path:?}),");
    rendered.push_str("    },\n");
    rendered.push(']');
    rendered.push('\n');
    rendered
}
