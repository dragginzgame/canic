mod canister_registry;

pub use canister_registry::*;

use crate::canister::CanisterType;

///
/// Canister
///

#[derive(Clone, Debug)]
pub struct Canister {
    pub attributes: Attributes,
    pub wasm: &'static [u8],
}

///
/// CanisterView
/// the front-facing version
///

#[derive(Clone, Debug)]
pub struct CanisterView {
    pub attributes: Attributes,
    pub wasm_size: usize,
}

impl From<&Canister> for CanisterView {
    fn from(canister: &Canister) -> Self {
        Self {
            attributes: canister.attributes.clone(),
            wasm_size: canister.wasm.len(),
        }
    }
}

///
/// Attributes
///
/// auto_create : number of canisters to create on root
///

#[derive(Clone, Debug, Default)]
pub struct Attributes {
    pub auto_create: Option<u16>,
    pub directory: DirectoryPolicy,
}

impl Attributes {
    #[must_use]
    pub const fn uses_directory(&self) -> bool {
        self.directory.uses_directory()
    }
}

///
/// DirectoryPolicy
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DirectoryPolicy {
    #[default]
    None,
    Limited(u16),
    Unlimited,
}

impl DirectoryPolicy {
    #[must_use]
    pub const fn uses_directory(self) -> bool {
        !matches!(self, Self::None)
    }

    #[must_use]
    pub const fn is_unlimited(self) -> bool {
        matches!(self, Self::Unlimited)
    }

    #[must_use]
    pub const fn limit(self) -> Option<u16> {
        if let Self::Limited(n) = self {
            Some(n)
        } else {
            None
        }
    }
}
