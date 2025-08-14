// icu_build
#[macro_export]
macro_rules! icu_build {
    ($file:expr) => {
        use std::{fs::File, io::Write, path::PathBuf};

        let config_str = include_str!($file);
        $crate::config::Config::init_from_toml(config_str).unwrap()
    };
}
