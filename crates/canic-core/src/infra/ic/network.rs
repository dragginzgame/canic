use std::fmt::{self, Display};

///
/// Network
/// Identifies the environment the canister believes it runs in.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Network {
    Ic,
    Local,
}

impl Network {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ic => "ic",
            Self::Local => "local",
        }
    }
}

impl Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
