use std::path::PathBuf;

fn main() {
    // If the env var changes, we must re-run to pick up a different config.
    println!("cargo:rerun-if-env-changed=CANIC_CONFIG_PATH");

    // Path to this crate at build time.
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set"));

    // Repo-default config location (works in workspace builds).
    // NOTE: This path will NOT exist in `cargo package` / `cargo publish` builds because
    // Cargo builds from a staged directory under target/package/ that does not include
    // workspace siblings.
    let repo_cfg = manifest_dir.join("../canisters/canic.toml");

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
        // Packaged builds won't include ../canisters. Skip build-time validation.
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
    canic_core::config::Config::init_from_toml(&config_str).expect("Invalid Canic config");

    // Export the canonicalized path for compile-time use.
    println!(
        "cargo:rustc-env=CANIC_CONFIG_PATH={}",
        cfg_path
            .canonicalize()
            .expect("canonicalize canic config path")
            .display()
    );
}
