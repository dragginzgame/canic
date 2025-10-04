#[macro_export]
macro_rules! canic_build {
    ($file:expr) => {{
        $crate::__canic_build_internal! {
            $file,
            |cfg_str, cfg_path, cfg| {
                use canic::{log, Log, types::CanisterType};
                use std::path::PathBuf;

                // Infer canister name from directory structure: .../canisters/<name>/...
                let canister_dir = {
                    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

                    manifest_dir
                        .ancestors()
                        .find(|p| p.file_name().map_or(false, |n| n == "canisters"))
                        .and_then(|canisters_dir| manifest_dir.strip_prefix(canisters_dir).ok())
                        .and_then(|rel| rel.components().next())
                        .and_then(|c| c.as_os_str().to_str())
                        .map(|s| s.to_string())
                        .or_else(|| std::env::var("CARGO_BIN_NAME").ok())
                        .or_else(|| std::env::var("CARGO_PKG_NAME").ok())
                        .expect("cannot infer canister name; place crate under canisters/<name>/ or set CARGO_BIN_NAME")
                };

                // Canister Lookup
                let dir = canister_dir.clone();
                if let Ok(canister_cfg) = cfg.try_get_canister(&dir.clone().into()) {
                    // canister capabilities
                    if canister_cfg.delegation {
                        println!("cargo:rustc-cfg=canic_capability_delegation");
                    }
                    if canister_cfg.scaling.is_some() {
                        println!("cargo:rustc-cfg=canic_capability_scaling");
                    }
                    if canister_cfg.sharding.is_some() {
                        println!("cargo:rustc-cfg=canic_capability_sharding");
                    }
                } else {
                    log!(
                        Log::Warn,
                        "⚠️ canister '{dir}' not found in Canic config. \
                        Add it under [canisters] in your config TOML."
                    );
                }

            }
        }
    }};
}

#[macro_export]
macro_rules! canic_build_root {
    ($file:expr) => {{
        $crate::__canic_build_internal! {
            $file,
            |_cfg_str, _cfg_path, _cfg| {
                // Mark this build as the root canister
                println!("cargo:rustc-cfg=canic_root");
            }
        }
    }};
}

// Internal helper — not exported
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_build_internal {
    ($file:expr, |$cfg_str:ident, $cfg_path:ident, $cfg:ident| $body:block) => {{
        // Use the workspace root so every crate gets the same base
        let ws_root = std::env::var("CARGO_WORKSPACE_ROOT")
            .unwrap_or_else(|_| std::env::var("CARGO_MANIFEST_DIR").unwrap());

        let $cfg_path = std::path::PathBuf::from(ws_root).join($file);

        // check config file exists (fails the build early if invalid)
        let $cfg_str = std::fs::read_to_string(&$cfg_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", $cfg_path.display(), e));

        // Init Config
        let $cfg = canic::config::Config::init_from_toml(&$cfg_str)
            .expect("Invalid Canic config");

        // declare the cfg names
        println!("cargo:rustc-check-cfg=cfg(canic)");
        println!("cargo:rustc-check-cfg=cfg(canic_capability_delegation)");
        println!("cargo:rustc-check-cfg=cfg(canic_capability_scaling)");
        println!("cargo:rustc-check-cfg=cfg(canic_capability_sharding)");
        println!("cargo:rustc-check-cfg=cfg(canic_github_ci)");
        println!("cargo:rustc-check-cfg=cfg(canic_root)");

        // everything gets the top level marker
        println!("cargo:rustc-cfg=canic");

        // Auto-enable the cfg when running under GitHub Actions.
        if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
            println!("cargo:rustc-cfg=canic_github_ci");
        }

        // Run the extra body (per-canister or nothing)
        $body

        // Export an ABSOLUTE path for include_str!
        let abs = $cfg_path.canonicalize().expect("canonicalize canic config path");
        println!("cargo:rustc-env=CANIC_CONFIG_PATH={}", abs.display());
        println!("cargo:rerun-if-changed={}", abs.display());
    }};
}
