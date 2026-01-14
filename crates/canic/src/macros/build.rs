// -----------------------------------------------------------------------------
// Build macros
// -----------------------------------------------------------------------------

/// Embed the shared Canic configuration into a canister crate's build script.
///
/// Reads the provided TOML file (relative to the crate manifest dir), validates it
/// using [`Config`](crate::__internal::core::config::Config), and sets
/// `CANIC_CONFIG_PATH` for later use by `include_str!`. Canister crates typically
/// invoke this from `build.rs`.
#[macro_export]
macro_rules! build {
    ($file:expr) => {{
        $crate::__canic_build_internal! {
            $file,
            |cfg_str, cfg_path, cfg| {
                let _ = (&cfg_str, &cfg_path, &cfg);
            }
        }
    }};
}

/// Embed the shared configuration for the root orchestrator canister.
///
/// Performs the same validation as [`macro@build`].
#[macro_export]
macro_rules! build_root {
    ($file:expr) => {{
        $crate::__canic_build_internal! {
            $file,
            |_cfg_str, _cfg_path, _cfg| {}
        }
    }};
}

/// Internal helper shared by [`macro@build`] and [`macro@build_root`].
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_build_internal {
    ($file:expr, |$cfg_str:ident, $cfg_path:ident, $cfg:ident| $body:block) => {{
        let manifest_dir =
            std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
        let $cfg_path = std::path::PathBuf::from(manifest_dir).join($file);
        println!("cargo:rerun-if-changed={}", $cfg_path.display());
        if let Some(parent) = $cfg_path.parent() {
            println!("cargo:rerun-if-changed={}", parent.display());
        }

        let $cfg_str = match std::fs::read_to_string(&$cfg_path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                panic!("Missing Canic config at {}", $cfg_path.display())
            }
            Err(e) => panic!("Failed to read {}: {}", $cfg_path.display(), e),
        };

        // Init Config
        let $cfg = $crate::__internal::core::bootstrap::init_config(&$cfg_str).expect("invalid canic config");

        // Run the extra body (per-canister or nothing)
        $body

        let abs = $cfg_path.canonicalize().expect("canonicalize canic config path");
        println!("cargo:rustc-env=CANIC_CONFIG_PATH={}", abs.display());
        println!("cargo:rerun-if-changed={}", abs.display());
    }};
}
