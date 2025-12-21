fn main() {
    println!("cargo:rerun-if-env-changed=DFX_NETWORK");

    let network = std::env::var("DFX_NETWORK").unwrap_or_else(|_| {
        panic!(
            "DFX_NETWORK must be set at build time (expected 'local' or 'ic'). \
             If building with cargo directly, set it explicitly, e.g. \
             DFX_NETWORK=local cargo build"
        )
    });

    match network.as_str() {
        "local" | "ic" => {}
        other => {
            panic!("DFX_NETWORK must be 'local' or 'ic' (got '{other}')");
        }
    }

    // Export for compile-time access via env!/option_env!
    println!("cargo:rustc-env=DFX_NETWORK={network}");
}
