fn main() {
    // Mark `canic_internal` cfg for this crate
    println!("cargo:rustc-check-cfg=cfg(canic_internal)");
    println!("cargo:rustc-cfg=canic_internal");
}
