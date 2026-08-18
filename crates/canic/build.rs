include!("src/build_support/cfg_catalog.rs");

fn main() {
    // The exported role macros consume this exact cfg catalog.
    for custom_cfg in CANIC_CUSTOM_CFG_NAMES {
        println!("cargo:rustc-check-cfg=cfg({custom_cfg})");
    }
}
