mod registry;

pub use registry::*;

use thiserror::Error as ThisError;

///
/// CanisterError
///

#[derive(Debug, ThisError)]
pub enum CanisterError {
    #[error(transparent)]
    CanisterRegistryError(#[from] CanisterRegistryError),
}

///
/// Canister
///

#[derive(Clone, Debug)]
pub struct Canister {
    pub kind: &'static str,
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
    pub indexing: IndexingPolicy,
}

impl Attributes {
    #[must_use]
    pub const fn is_indexable(&self) -> bool {
        self.indexing.is_indexable()
    }
}

///
/// IndexingPolicy
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum IndexingPolicy {
    #[default]
    None,
    Limited(u16),
    Unlimited,
}

impl IndexingPolicy {
    #[must_use]
    pub const fn is_indexable(self) -> bool {
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
