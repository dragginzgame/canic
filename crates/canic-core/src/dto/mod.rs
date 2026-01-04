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
//! - DTOs therefore prefer simple list representations (`Vec<T>`). When keyed
//!   data is needed, define small entry structs (`FooEntryView`) instead of
//!   tuples or maps. More structured shapes are used only when they materially
//!   reduce complexity for consumers, not to mirror storage internals or
//!   reintroduce lost semantics.
//!
//! - Avoid `HashMap` in DTOs. Keyed semantics and ordering are not preserved at
//!   the boundary; use `Vec<...EntryView>` or explicit list types instead.
//!
//! In short: stable storage is authoritative; DTOs describe how data is
//! transported, not what guarantees it provides.

pub mod abi;
pub mod canister;
pub mod cascade;
pub mod cycles;
pub mod env;
pub mod error;
pub mod http;
pub mod log;
pub mod memory;
pub mod metrics;
pub mod page;
pub mod placement;
pub mod pool;
pub mod rpc;
pub mod state;
pub mod subnet;
pub mod topology;

///
/// Prelude
///

pub mod prelude {
    pub use crate::ids::{CanisterRole, SubnetRole};
    pub use candid::{CandidType, Nat, Principal};
    pub use serde::{Deserialize, Serialize};
}
