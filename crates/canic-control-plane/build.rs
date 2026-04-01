fn main() {
    println!("cargo:rustc-check-cfg=cfg(canic_test_small_wasm_store)");
}
