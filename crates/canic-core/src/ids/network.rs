use std::fmt::{self, Display};

///
/// BuildNetwork
/// Identifies the environment the canister believes it runs in.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildNetwork {
    Ic,
    Local,
}

impl BuildNetwork {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ic => "ic",
            Self::Local => "local",
        }
    }
}

impl Display for BuildNetwork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
