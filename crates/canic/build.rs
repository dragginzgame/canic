fn main() {
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let repo_cfg = manifest_dir.join("../canisters/canic.toml");
    println!("cargo:rerun-if-changed={}", repo_cfg.display());
    if let Some(parent) = repo_cfg.parent() {
        println!("cargo:rerun-if-changed={}", parent.display());
    }

    let config_path = if repo_cfg.exists() {
        repo_cfg
            .canonicalize()
            .expect("canonicalize canic.toml in repo")
    } else {
        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let fallback = out_dir.join("canic.default.toml");

        std::fs::write(
            &fallback,
            r#"controllers = []
app_directory = []

[subnets.prime]
"#,
        )
        .expect("write default canic config");

        fallback
    };

    println!(
        "cargo:rustc-env=CANIC_CONFIG_PATH={}",
        config_path.display()
    );
    println!("cargo:rerun-if-changed={}", config_path.display());
}
