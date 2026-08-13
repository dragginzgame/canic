include!("src/build_support/cfg_catalog.rs");

fn main() {
    // The exported endpoint macros use these cfg names when optional endpoint
    // groups are compiled out for a role-specific canister build.
    for custom_cfg in CANIC_CUSTOM_CFG_NAMES {
        println!("cargo:rustc-check-cfg=cfg({custom_cfg})");
    }
    println!("cargo:rerun-if-env-changed=CANIC_INTERNAL_TEST_ENDPOINTS");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_METRICS");

    if std::env::var_os("CANIC_INTERNAL_TEST_ENDPOINTS").is_none() {
        // Default builds ship the slimmer demo/reference surface; internal
        // observability and topology test endpoints opt back in explicitly.
        println!("cargo:rustc-cfg=canic_disable_bundle_observability_env");
    }
    if std::env::var_os("CARGO_FEATURE_METRICS").is_none() {
        println!("cargo:rustc-cfg=canic_disable_bundle_metrics");
    }
}
