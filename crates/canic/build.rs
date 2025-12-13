fn main() {
    // Path to the current crate (crates/canic)
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    // Expected location of the repo-level Canic configuration
    //
    // This is used for:
    // - examples
    // - tests
    // - non-canister builds of the `canic` crate
    let repo_cfg = manifest_dir.join("../canisters/canic.toml");

    // Re-run if the config file appears, disappears, or changes
    println!("cargo:rerun-if-changed={}", repo_cfg.display());
    if let Some(parent) = repo_cfg.parent() {
        println!("cargo:rerun-if-changed={}", parent.display());
    }

    // Resolve the config path:
    // - Prefer the real repo config if it exists
    // - Otherwise generate a minimal, valid fallback config
    let config_path = if repo_cfg.exists() {
        repo_cfg
            .canonicalize()
            .expect("canonicalize canic.toml in repo")
    } else {
        // Fallback mode: generate a minimal config so that
        // macros using `include_str!(env!(\"CANIC_CONFIG_PATH\"))`
        // can still compile in examples and tests.
        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let fallback = out_dir.join("canic.default.toml");

        std::fs::write(
            &fallback,
            "controllers = []
app_directory = []

[subnets.prime]
",
        )
        .expect("write default canic config");

        fallback
    };

    // Export the config path as a compile-time environment variable.
    //
    // This is consumed by Canic lifecycle macros via:
    // `include_str!(env!("CANIC_CONFIG_PATH"))`.
    println!(
        "cargo:rustc-env=CANIC_CONFIG_PATH={}",
        config_path.display()
    );

    // Ensure rebuild if the selected config changes
    println!("cargo:rerun-if-changed={}", config_path.display());
}
