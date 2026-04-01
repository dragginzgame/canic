fn main() {
    const ENV_NAME: &str = "CANIC_IMPLICIT_WASM_STORE_MAX_STORE_BYTES";
    const DEFAULT_MAX_STORE_BYTES: u64 = 40_000_000;

    println!("cargo:rerun-if-env-changed={ENV_NAME}");

    let resolved = std::env::var(ENV_NAME)
        .map(|value| {
            value
                .parse::<u64>()
                .unwrap_or_else(|err| panic!("{ENV_NAME} must parse as u64, got '{value}': {err}"))
        })
        .unwrap_or(DEFAULT_MAX_STORE_BYTES);

    println!("cargo:rustc-env={ENV_NAME}={resolved}");
}
