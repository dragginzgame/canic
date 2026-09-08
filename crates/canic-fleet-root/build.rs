fn main() {
    let config_path_env = canic::__internal::core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV;
    println!("cargo:rerun-if-env-changed={config_path_env}");

    let config_path = std::env::var(config_path_env).unwrap_or_else(|_| "canic.toml".to_string());

    canic::build!(config_path);
}
