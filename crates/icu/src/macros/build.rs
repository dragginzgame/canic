#[macro_export]
macro_rules! icu_build {
    () => {};

    ($file:expr) => {{
        // Compile-time include; if the path is wrong, you'll get a hard compile error.
        const __ICU_CFG: &str = include_str!($file);

        // Validate/initialize your config at build time.
        // If parsing fails, build.rs will fail the build (good signal).
        $crate::config::Config::init_from_toml(__ICU_CFG)
            .expect("invalid TOML in icu_build! config");
    }};
}
