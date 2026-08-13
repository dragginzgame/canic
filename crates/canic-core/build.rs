fn main() {
    println!("cargo:rerun-if-env-changed=ICP_ENVIRONMENT");

    let network = std::env::var("ICP_ENVIRONMENT").unwrap_or_else(|_| {
        // Explicit, intentional default for local development.
        "local".to_string()
    });

    match network.as_str() {
        "local" | "ic" => {
            println!("cargo:rustc-env=ICP_ENVIRONMENT={network}");
        }
        other => {
            panic!(
                "ICP_ENVIRONMENT is invalid.\n\
Expected: local or ic\n\
Got: '{other}'\n\
Hint: unset ICP_ENVIRONMENT to default to 'local'."
            );
        }
    }
}
