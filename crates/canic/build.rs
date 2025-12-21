fn main() {
    println!("cargo:rerun-if-env-changed=CANIC_CONFIG_PATH");

    let manifest_dir = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set"),
    );

    let repo_cfg = manifest_dir.join("../canisters/canic.toml");

    let cfg_path = match std::env::var("CANIC_CONFIG_PATH") {
        Ok(val) => {
            let path = std::path::PathBuf::from(val);
            if path.is_relative() {
                manifest_dir.join(path)
            } else {
                path
            }
        }
        Err(_) => repo_cfg,
    };

    // Re-run if the config file changes
    println!("cargo:rerun-if-changed={}", cfg_path.display());
    if let Some(parent) = cfg_path.parent() {
        println!("cargo:rerun-if-changed={}", parent.display());
    }

    // Ensure config exists
    assert!(
        cfg_path.exists(),
        "Missing Canic config at {}",
        cfg_path.display()
    );

    let config_str = std::fs::read_to_string(&cfg_path).expect("read canic config for validation");
    canic_core::config::Config::init_from_toml(&config_str).expect("Invalid Canic config");

    // Export canonicalized path for macros
    println!(
        "cargo:rustc-env=CANIC_CONFIG_PATH={}",
        cfg_path
            .canonicalize()
            .expect("canonicalize canic config path")
            .display()
    );
}
