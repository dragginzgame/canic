//! Private Canic test harness and separately selectable integration suites.
//!
//! The library owns Canic-specific fixtures, Root baselines and test seams.
//! Reusable IC testing mechanics remain with `ic-testkit`.

pub mod canister;
pub mod embedded_root;
pub mod payload_contract;
pub mod pic;
#[cfg(any(not(test), feature = "governed-pocketic-tests"))]
pub mod root;
pub mod workspace;
