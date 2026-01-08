//!
//! Canonical representations of external IC specs (ICRC, NNS, SNS, etc.).
//! This module corrals the verbose candid bindings so the rest of the codebase
//! can import clean wrappers with consistent naming.
//!

pub mod governance;
pub mod standards;
pub mod system;

/// Shared imports for spec modules so type definitions stay concise.
pub mod prelude {
    pub use crate::{
        candid::{CandidType, Principal},
        types::{Account, Int, Nat, Subaccount},
    };
    pub use serde::{Deserialize, Serialize};
    pub use serde_bytes::ByteBuf;
}
