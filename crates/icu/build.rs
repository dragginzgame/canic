fn main() {
    // Mark `icu_internal` cfg for this crate
    println!("cargo:rustc-check-cfg=cfg(icu_internal)");
    println!("cargo:rustc-cfg=icu_internal");
}
