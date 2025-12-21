fn main() {
    // Re-run this build script if the build-time network changes.
    // This ensures downstream code sees the correct value via env!/option_env!.
    println!("cargo:rerun-if-env-changed=DFX_NETWORK");

    match std::env::var("DFX_NETWORK") {
        // Valid, explicit network: propagate it as a compile-time environment variable.
        //
        // Library crates do not *require* this, but if present and valid we make it
        // available so dependent crates can read a consistent build-time value.
        Ok(val) if val == "local" || val == "ic" => {
            println!("cargo:rustc-env=DFX_NETWORK={val}");
        }

        // Invalid value provided: warn, but do not fail the build.
        //
        // This keeps library builds usable under raw Cargo while clearly signaling
        // that top-level canisters are expected to enforce correctness.
        Ok(other) => {
            println!(
                "cargo:warning=Invalid DFX_NETWORK='{other}'; expected 'local' or 'ic'. \
                 Downstream canisters must enforce this explicitly.",
            );
        }

        // No network specified: assume a library/dependency build.
        //
        // Cargo does not provide DFX_NETWORK by default, and dependency crates must
        // remain buildable in isolation. Canister/root crates are responsible for
        // enforcing a concrete network at their build boundary.
        Err(_) => {
            println!(
                "cargo:warning=DFX_NETWORK not set; assuming library/dependency build. \
                 Canister crates must enforce this at their boundary."
            );
        }
    }
}
