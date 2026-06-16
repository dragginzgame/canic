//! Module: cdk::spec
//!
//! Responsibility: canonical representations of external IC specs.
//! Does not own: business interpretation of ICRC, NNS, or ledger behavior.
//! Boundary: corrals verbose Candid bindings behind consistently named wrappers.

pub mod standards;

/// Shared imports for spec modules so type definitions stay concise.
pub mod prelude {
    pub use crate::cdk::{
        candid::{CandidType, Principal},
        types::{Account, Int, Nat, Subaccount},
    };
    pub use serde::{Deserialize, Serialize};
    pub use serde_bytes::ByteBuf;
}
