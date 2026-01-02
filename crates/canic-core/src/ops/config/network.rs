use crate::infra::ic as infra_ic;
use core::fmt;

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

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[must_use]
pub fn build_network() -> Option<Network> {
    infra_ic::build_network().map(Network::from)
}

impl From<infra_ic::Network> for Network {
    fn from(value: infra_ic::Network) -> Self {
        match value {
            infra_ic::Network::Ic => Self::Ic,
            infra_ic::Network::Local => Self::Local,
        }
    }
}
