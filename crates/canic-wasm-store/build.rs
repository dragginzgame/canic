fn main() {
    println!("cargo:rerun-if-env-changed=CANIC_CONFIG_PATH");

    let config_path =
        std::env::var("CANIC_CONFIG_PATH").unwrap_or_else(|_| "canic.toml".to_string());

    canic::build!(config_path);
}
