#[macro_export]
macro_rules! icu_build {
    // No-config form: just set the CI cfg if we're in GitHub Actions.
    () => {
        $crate::icu_build!(@common);
    };

    // With-config form: include the file at compile time (errors if missing),
    // parse/validate it now, then run the common CI bits.
    ($file:expr) => {{
        // Compile-time include; if the path is wrong, you'll get a hard compile error.
        const __ICU_CFG: &str = include_str!($file);

        // Validate/initialize your config at build time.
        // If parsing fails, build.rs will fail the build (good signal).
        $crate::config::Config::init_from_toml(__ICU_CFG)
            .expect("invalid TOML in icu_build! config");

        $crate::icu_build!(@common);
    }};

    // Internal shared logic
    (@common) => {{
        // Tell rustc that `icu_github_ci` is a valid cfg to avoid warnings.
        println!("cargo::rustc-check-cfg=cfg(icu_github_ci)");

        // Auto-enable the cfg when running under GitHub Actions.
        if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
            println!("cargo:rustc-cfg=icu_github_ci");
        }
    }};
}
