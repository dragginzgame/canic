//! DTO boundary definitions.
//!
//! ## Design rules
//!
//! - DTOs are **passive transport types**. They contain no domain logic, policy,
//!   validation, normalization, or invariant enforcement.
//!
//! - DTOs do **not claim guarantees that Candid cannot enforce**. In particular,
//!   uniqueness, ordering, and keyed semantics are not preserved at the wire
//!   level and must not be relied upon.
//!
//! - All authoritative invariants (e.g. ownership, uniqueness, replacement
//!   semantics) live in **storage and ops layers**, never in DTOs.
//! - Data exported from stable registries is treated as a **snapshot** of state,
//!   not a live or authoritative registry.
//!
//! - DTOs therefore prefer simple list representations (`Vec<T>` or
//!   `Vec<(K, V)>`). More structured shapes are used only when they materially
//!   reduce complexity for consumers, not to mirror storage internals or
//!   reintroduce lost semantics.
//!
//! In short: stable storage is authoritative; DTOs describe how data is
//! transported, not what guarantees it provides.

pub mod abi;
pub mod canister;
pub mod directory;
pub mod env;
pub mod log;
pub mod metrics;
pub mod page;
pub mod placement;
pub mod pool;
pub mod registry;
pub mod rpc;
pub mod snapshot;
pub mod state;
pub mod subnet;

///
/// PRELUDE
///

pub mod prelude {
    pub use crate::ids::{CanisterRole, SubnetRole};
    pub use candid::{CandidType, Principal};
    pub use derive_more::Display;
    pub use serde::{Deserialize, Serialize};
    pub use std::collections::HashMap;
}
