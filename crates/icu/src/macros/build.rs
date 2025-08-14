#[macro_export]
macro_rules! icu_build {
    () => {
        $crate::icu_build!(@common);
    };

    ($file:expr) => {{
        // Use the workspace root so every crate gets the same base
        let ws_root = std::env::var("CARGO_WORKSPACE_ROOT")
            .unwrap_or_else(|_| std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let cfg_path = std::path::PathBuf::from(ws_root).join($file);

        // Re-run if config changes
        println!("cargo:rerun-if-changed={}", cfg_path.display());

        // Validate config now (fails the build early if invalid)
        let cfg_str = std::fs::read_to_string(&cfg_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {e}", cfg_path.display()));

        // Init config
        icu::config::Config::init_from_toml(&cfg_str)
            .expect("Invalid ICU config");

        // Pass the path to lib.rs so it can also include_str! without hardcoding
        println!("cargo:rustc-env=ICU_CONFIG_PATH={}", cfg_path.display());

        $crate::icu_build!(@common);
    }};

    // Internal shared logic
    (@common) => {{
        //
        // Set the icu_github_ci flag so you can conditionally execute
        // code if it's in CI
        //

        // Tell rustc that `icu_github_ci` is a valid cfg to avoid warnings.
        println!("cargo::rustc-check-cfg=cfg(icu_github_ci)");

        // Auto-enable the cfg when running under GitHub Actions.
        if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
            println!("cargo:rustc-cfg=icu_github_ci");
        }
    }};
}
