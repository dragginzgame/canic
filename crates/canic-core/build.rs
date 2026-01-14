fn main() {
    let network = std::env::var("DFX_NETWORK").unwrap_or_else(|_| {
        // Explicit, intentional default for local development.
        "local".to_string()
    });

    match network.as_str() {
        "local" | "ic" => {
            println!("cargo:rustc-env=DFX_NETWORK={network}");
        }
        other => {
            panic!(
                "DFX_NETWORK is invalid.\n\
Expected: local or ic\n\
Got: '{other}'\n\
Hint: unset DFX_NETWORK to default to 'local'."
            );
        }
    }
}
