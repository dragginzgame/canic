fn main() {
    println!("cargo:rerun-if-env-changed=CANIC_MEMORY_BUCKET_PAGES");
    let bucket_pages = match std::env::var("CANIC_MEMORY_BUCKET_PAGES") {
        Ok(value) => value
            .parse::<u16>()
            .expect("CANIC_MEMORY_BUCKET_PAGES must be a nonzero u16 page count"),
        Err(std::env::VarError::NotPresent) => 16,
        Err(error) => panic!("invalid CANIC_MEMORY_BUCKET_PAGES: {error}"),
    };
    assert!(
        bucket_pages > 0,
        "CANIC_MEMORY_BUCKET_PAGES must be nonzero"
    );
    println!("cargo:rustc-env=CANIC_MEMORY_BUCKET_PAGES={bucket_pages}");
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
