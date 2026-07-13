use std::str::FromStr;

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
    // Return the cargo profile flags for one Canic canister build.
    #[must_use]
    pub(crate) const fn cargo_args(self) -> &'static [&'static str] {
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

impl FromStr for CanisterBuildProfile {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "debug" => Ok(Self::Debug),
            "fast" => Ok(Self::Fast),
            "release" => Ok(Self::Release),
            _ => Err(format!(
                "invalid build profile {value}; use debug, fast, or release"
            )),
        }
    }
}
