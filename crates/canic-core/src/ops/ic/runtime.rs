//! Runtime context helpers for IC execution.
//!
//! This module provides the approved IC surface for ambient context:
//! identity, time, scheduling, traps, and task spawning.

use crate::cdk;
use core::future::Future;

pub use crate::cdk::{
    candid::CandidType,
    types::{Cycles, Principal, TC},
};

/// Return the current canister principal.
#[must_use]
pub fn canister_self() -> Principal {
    cdk::api::canister_self()
}

/// Return the current caller principal.
#[must_use]
pub fn msg_caller() -> Principal {
    cdk::api::msg_caller()
}

/// Return the current IC time in nanoseconds since Unix epoch.
#[must_use]
pub fn time() -> u64 {
    cdk::api::time()
}

/// Return the current UNIX epoch time in seconds.
#[must_use]
pub fn now_secs() -> u64 {
    cdk::utils::time::now_secs()
}

/// Trap the canister with the provided message.
pub fn trap(message: &str) -> ! {
    cdk::api::trap(message)
}

/// Print a line to the IC debug output.
pub fn println(message: &str) {
    cdk::println!("{message}");
}

/// Spawn a task on the IC runtime.
pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    cdk::futures::spawn(future);
}
