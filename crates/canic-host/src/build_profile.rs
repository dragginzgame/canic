///
/// CanisterBuildProfile
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanisterBuildProfile {
    Debug,
    Fast,
    Release,
}

impl CanisterBuildProfile {
    // Resolve the current requested build profile from the explicit Canic wasm selector.
    #[must_use]
    pub fn current() -> Self {
        match std::env::var("CANIC_WASM_PROFILE").ok().as_deref() {
            Some("debug") => Self::Debug,
            Some("fast") => Self::Fast,
            _ => Self::Release,
        }
    }

    // Return the cargo profile flags for one Canic canister build.
    #[must_use]
    pub const fn cargo_args(self) -> &'static [&'static str] {
        match self {
            Self::Debug => &[],
            Self::Fast => &["--profile", "fast"],
            Self::Release => &["--release"],
        }
    }

    // Return the target-profile directory name for one Canic canister build.
    #[must_use]
    pub const fn target_dir_name(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Fast => "fast",
            Self::Release => "release",
        }
    }
}
