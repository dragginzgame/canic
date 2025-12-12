//!
//! Canonical representations of external IC specs (ICRC, NNS, SNS, etc.).
//! This module corrals the verbose candid bindings so the rest of the codebase
//! can import clean wrappers with consistent naming.
//!

pub mod ic;
pub mod icrc;
pub mod nns;
pub mod sns;

/// Shared imports for spec modules so type definitions stay concise.
pub mod prelude {
    pub use crate::{
        cdk::{
            candid::{CandidType, Principal},
            types::{Account, Int, Nat, Subaccount},
        },
        ids::CanisterRole,
    };
    pub use serde::Deserialize;
    pub use serde_bytes::ByteBuf;
}
