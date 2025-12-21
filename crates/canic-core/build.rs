use candid::Principal;

fn validate_principal(label: &str, text: &str) {
    if let Err(err) = Principal::from_text(text) {
        panic!("Invalid principal literal {label}: {text} ({err})");
    }
}

macro_rules! static_canisters {
    ($($name:ident = $id:expr;)+) => {{
        $(validate_principal(stringify!($name), $id);)+
    }};
}

macro_rules! sns_table {
    (
        $(
            $name:ident {
                root: $root:expr,
                governance: $gov:expr,
                index: $idx:expr,
                ledger: $led:expr $(,)?
            }
        ),+ $(,)?
    ) => {{
        $(
            validate_principal(concat!(stringify!($name), ".root"), $root);
            validate_principal(concat!(stringify!($name), ".governance"), $gov);
            validate_principal(concat!(stringify!($name), ".index"), $idx);
            validate_principal(concat!(stringify!($name), ".ledger"), $led);
        )+
    }};
}

fn main() {
    // Re-run this build script if the build-time network changes.
    // This ensures downstream code sees the correct value via env!/option_env!.
    println!("cargo:rerun-if-env-changed=DFX_NETWORK");
    println!("cargo:rerun-if-changed=src/env/ck.inc.rs");
    println!("cargo:rerun-if-changed=src/env/nns.inc.rs");
    println!("cargo:rerun-if-changed=src/env/sns.inc.rs");

    // Share the same principal data with build-time validation.
    include!("src/env/ck.inc.rs");
    include!("src/env/nns.inc.rs");
    include!("src/env/sns.inc.rs");

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
