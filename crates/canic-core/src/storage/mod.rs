//! Persisted record and stable-memory representation.
//!
//! This module contains passive persisted and cached representations. Model
//! owns authoritative state and storage invariants; ops owns access and
//! conversion at this boundary.
//!
//! Multi-step orchestration lives in `workflow`; pure decision helpers live in
//! `domain::policy::pure`.

pub mod canister;
pub mod stable;

///
/// Prelude
///

pub mod prelude {
    pub use crate::cdk::types::Principal;
    pub use crate::impl_storable_bounded;
    pub use canic_contracts::{
        cycles::Cycles, ids::CanisterRole, ids::ComponentSpecId, ids::ManagedCanisterBinding,
    };
    pub use serde::{Deserialize, Serialize};
}
