fn main() {
    // Rebuild when test-material cfg flag changes so cfg-gated endpoints stay aligned.
    println!("cargo:rerun-if-env-changed=CANIC_TEST_DELEGATION_MATERIAL");

    // Register and optionally enable the test-only delegation-material cfg.
    println!("cargo:rustc-check-cfg=cfg(canic_test_delegation_material)");
    if std::env::var_os("CANIC_TEST_DELEGATION_MATERIAL").is_some() {
        println!("cargo:rustc-cfg=canic_test_delegation_material");
    }

    canic::build!("canic.toml");
}
