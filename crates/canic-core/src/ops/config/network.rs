use crate::infra;
use std::fmt::{self, Display};

///
/// Network
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

impl From<infra::ic::Network> for Network {
    fn from(value: infra::ic::Network) -> Self {
        match value {
            infra::ic::Network::Ic => Self::Ic,
            infra::ic::Network::Local => Self::Local,
        }
    }
}

#[must_use]
pub fn build_network() -> Option<Network> {
    infra::ic::build_network().map(Network::from)
}
