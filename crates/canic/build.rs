fn main() {
    // The exported endpoint macros use these cfg names when optional endpoint
    // groups are compiled out for a role-specific canister build.
    println!("cargo:rustc-check-cfg=cfg(canic_role_attestation_refresh)");
    println!("cargo:rustc-check-cfg=cfg(canic_delegated_tokens_enabled)");
    println!("cargo:rustc-check-cfg=cfg(canic_delegated_token_issuer)");
    println!("cargo:rustc-check-cfg=cfg(canic_icrc21_enabled)");
    println!("cargo:rustc-check-cfg=cfg(canic_is_root)");
    println!("cargo:rustc-check-cfg=cfg(canic_has_scaling)");
    println!("cargo:rustc-check-cfg=cfg(canic_has_sharding)");
    println!("cargo:rustc-check-cfg=cfg(canic_has_root_wasm_store_bootstrap_release_set)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_icrc_standards)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_metadata)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_observability_env)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_observability_log)");
    println!("cargo:rustc-check-cfg=cfg(canic_memory_ledger_enabled)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_metrics)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_cycle_tracker)");
    println!("cargo:rustc-check-cfg=cfg(canic_metrics_core)");
    println!("cargo:rustc-check-cfg=cfg(canic_metrics_placement)");
    println!("cargo:rustc-check-cfg=cfg(canic_metrics_platform)");
    println!("cargo:rustc-check-cfg=cfg(canic_metrics_runtime)");
    println!("cargo:rustc-check-cfg=cfg(canic_metrics_security)");
    println!("cargo:rustc-check-cfg=cfg(canic_metrics_storage)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_auth_attestation)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_state)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_index)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_children)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_topology_placement)");
    println!("cargo:rustc-check-cfg=cfg(canic_disable_bundle_nonroot_sync_topology)");
    println!("cargo:rerun-if-env-changed=CANIC_INTERNAL_TEST_ENDPOINTS");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_METRICS");

    if std::env::var_os("CANIC_INTERNAL_TEST_ENDPOINTS").is_none() {
        // Default builds ship the slimmer demo/reference surface; internal
        // observability and topology test endpoints opt back in explicitly.
        println!("cargo:rustc-cfg=canic_disable_bundle_observability_env");
        println!("cargo:rustc-cfg=canic_disable_bundle_topology_index");
    }
    if std::env::var_os("CARGO_FEATURE_METRICS").is_none() {
        println!("cargo:rustc-cfg=canic_disable_bundle_metrics");
    }
}
