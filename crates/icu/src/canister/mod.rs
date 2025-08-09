mod registry;

pub use registry::*;

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// CanisterError
///

#[derive(Debug, ThisError)]
pub enum CanisterError {
    #[error("canister '{0}' not found")]
    CanisterNotFound(String),
}

///
/// Canister
///

#[derive(Clone, Debug)]
pub struct Canister {
    pub kind: &'static str,
    pub attributes: CanisterAttributes,
    pub wasm: &'static [u8],
}

///
/// CanisterInfo
/// the front-facing version
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterInfo {
    pub attributes: CanisterAttributes,
    pub wasm_size: usize,
}

impl From<&Canister> for CanisterInfo {
    fn from(canister: &Canister) -> Self {
        Self {
            attributes: canister.attributes.clone(),
            wasm_size: canister.wasm.len(),
        }
    }
}

///
/// CanisterAttributes
///
/// auto_create : number of canisters to create on root
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterAttributes {
    pub auto_create: Option<u16>,
    pub indexable: Option<CanisterIndexable>,
}

impl CanisterAttributes {
    #[must_use]
    pub fn is_indexable(&self) -> bool {
        self.indexable.is_some()
    }
}

///
/// CanisterIndexable
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum CanisterIndexable {
    Limited(u16),
    Unlimited,
}

impl CanisterIndexable {
    #[must_use]
    pub const fn singleton() -> Self {
        Self::Limited(1)
    }

    #[must_use]
    pub const fn limited(limit: u16) -> Self {
        Self::Limited(limit)
    }

    #[must_use]
    pub const fn unlimited() -> Self {
        Self::Unlimited
    }
}
