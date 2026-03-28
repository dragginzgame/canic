use std::env;

fn main() {
    // Rebuild when test-material cfg flag changes so cfg-gated APIs stay aligned.
    println!("cargo:rerun-if-env-changed=CANIC_TEST_DELEGATION_MATERIAL");

    // Register and optionally enable the test-only delegation-material cfg.
    // This stays disabled in normal builds unless explicitly requested.
    println!("cargo:rustc-check-cfg=cfg(canic_test_delegation_material)");
    println!("cargo:rustc-check-cfg=cfg(canic_is_root)");
    if env::var_os("CANIC_TEST_DELEGATION_MATERIAL").is_some() {
        println!("cargo:rustc-cfg=canic_test_delegation_material");
    }

    let network = env::var("DFX_NETWORK").unwrap_or_else(|_| {
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
