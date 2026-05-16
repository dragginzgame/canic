///
/// Opinionated prelude for Canic canister crates.
///
/// Prefer importing from the prelude in your canister `lib.rs` to keep endpoint
/// modules small and consistent. Library crates and shared modules should
/// generally import from specific paths instead of pulling in the entire prelude.
///
pub use crate::{
    api::{
        canister::CanisterRole,
        ic::Call,
        ops::{log, perf},
        timer::{timer, timer_interval},
    },
    cdk::{
        api::{canister_self, msg_caller},
        candid::CandidType,
    },
    dto::auth::DelegatedToken,
};

pub use canic_macros::{canic_query, canic_update};
