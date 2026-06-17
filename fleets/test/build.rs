fn main() {
    println!("cargo:rerun-if-env-changed=CANIC_TEST_DELEGATION_MATERIAL");
    println!("cargo:rustc-check-cfg=cfg(canic_test_delegation_material)");
    if std::env::var_os("CANIC_TEST_DELEGATION_MATERIAL").is_some() {
        println!("cargo:rustc-cfg=canic_test_delegation_material");
    }

    canic::build!("../canic.toml");
}
