///
/// Opinionated prelude for Canic canister crates.
///
/// Prefer importing from the prelude in your canister `lib.rs` to keep endpoint
/// modules small and consistent. Library crates and shared modules should
/// generally import from specific paths instead of pulling in the entire prelude.
///
pub use crate::api::{
    canister::CanisterRole,
    ops::{log, perf},
};

pub use canic_macros::{query, update};
