use std::path::PathBuf;

fn main() {
    // The exported endpoint macros use these cfg names when optional endpoint
    // groups are compiled out for a role-specific canister build.
    println!("cargo:rustc-check-cfg=cfg(canic_accepts_delegation_signer_proof)");
    println!("cargo:rustc-check-cfg=cfg(canic_accepts_delegation_verifier_proof)");
    println!("cargo:rustc-check-cfg=cfg(canic_delegated_tokens_enabled)");
    println!("cargo:rustc-check-cfg=cfg(canic_icrc21_enabled)");
    println!("cargo:rustc-check-cfg=cfg(canic_is_root)");
    println!("cargo:rustc-check-cfg=cfg(canic_has_scaling)");
    println!("cargo:rustc-check-cfg=cfg(canic_has_sharding)");
    println!("cargo:rustc-check-cfg=cfg(canic_has_root_wasm_store_bootstrap_release_set)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_standards_icrc)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_standards_canic)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_observability_memory)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_observability_env)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_observability_log)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_metrics)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_auth_attestation)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_state)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_directory)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_children)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_cycles)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_placement)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_nonroot_sync_topology)");

    // If the env var changes, we must re-run to pick up a different config.
    println!("cargo:rerun-if-env-changed=CANIC_CONFIG_PATH");

    // Path to this crate at build time.
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set"));

    // Repo-default config location (works in workspace builds).
    // NOTE: This path will NOT exist in `cargo package` / `cargo publish` builds because
    // Cargo builds from a staged directory under target/package/ that does not include
    // workspace siblings.
    let repo_cfg = manifest_dir.join("../../canisters/canic.toml");

    // Determine the config path:
    // - If CANIC_CONFIG_PATH is set, it is authoritative (relative paths resolved from the crate).
    // - Otherwise, fall back to the repo default.
    let env_cfg = std::env::var("CANIC_CONFIG_PATH").ok();
    let cfg_path = env_cfg.as_ref().map_or(repo_cfg, |val| {
        let path = PathBuf::from(val);
        if path.is_relative() {
            manifest_dir.join(path)
        } else {
            path
        }
    });

    // If the user explicitly set CANIC_CONFIG_PATH, missing config is a hard error.
    if env_cfg.is_some() {
        assert!(
            cfg_path.exists(),
            "Missing Canic config at {}",
            cfg_path.display()
        );
    } else if !cfg_path.exists() {
        // Packaged builds won't include ../../canisters. Skip build-time validation.
        // This keeps `cargo package` / `cargo publish` working.
        //
        // IMPORTANT: Any code that consumes CANIC_CONFIG_PATH must tolerate it being unset
        // in packaged builds (e.g., via option_env!).
        println!(
            "cargo:warning=CANIC_CONFIG_PATH not set and default config not found at {}; \
             skipping config validation (likely a packaged build)",
            cfg_path.display()
        );
        return;
    }

    // Re-run if the config changes.
    println!("cargo:rerun-if-changed={}", cfg_path.display());
    if let Some(parent) = cfg_path.parent() {
        // Directory watch is not recursive, but it helps in common cases (file replaced/renamed).
        println!("cargo:rerun-if-changed={}", parent.display());
    }

    // Validate the config early so failures are caught at build time.
    let config_str = std::fs::read_to_string(&cfg_path).expect("read canic config for validation");
    let config =
        canic_core::bootstrap::parse_config_model(&config_str).expect("invalid canic config");
    let compact_config = canic_core::bootstrap::compact_config_source(&config_str);
    let compiled_config = canic_core::bootstrap::emit_config_model_source(&config);

    // Emit the same generated artifacts that downstream canister build scripts use so
    // examples inside the facade crate can compile with `start!` / `start_root!`.
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR must be set"));
    let compact_cfg_path = out_dir.join("canic.compact.toml");
    let compiled_cfg_path = out_dir.join("canic.compiled.rs");
    std::fs::write(&compact_cfg_path, compact_config).expect("write compact canic config");
    std::fs::write(&compiled_cfg_path, compiled_config).expect("write compiled canic config");

    // Export the canonicalized paths for compile-time use.
    let compact_abs = compact_cfg_path
        .canonicalize()
        .expect("canonicalize compact canic config path");
    let compiled_abs = compiled_cfg_path
        .canonicalize()
        .expect("canonicalize compiled canic config path");
    let source_abs = cfg_path
        .canonicalize()
        .expect("canonicalize canic config path");

    println!("cargo:rustc-env=CANIC_CONFIG_PATH={}", source_abs.display());
    println!(
        "cargo:rustc-env=CANIC_CONFIG_SOURCE_PATH={}",
        compact_abs.display()
    );
    println!(
        "cargo:rustc-env=CANIC_CONFIG_MODEL_PATH={}",
        compiled_abs.display()
    );
}
