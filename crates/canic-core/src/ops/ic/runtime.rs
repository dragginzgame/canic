//! Runtime context helpers for IC execution.
//!
//! This module provides the approved IC surface for ambient context:
//! identity, time, scheduling, traps, and task spawning.

use crate::cdk;
use core::future::Future;

pub use crate::cdk::candid::CandidType;
pub use crate::cdk::types::{Cycles, Principal, TC};

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

/// Return the current UNIX epoch time in milliseconds.
#[must_use]
pub fn now_millis() -> u64 {
    cdk::utils::time::now_millis()
}

/// Return the current UNIX epoch time in microseconds.
#[must_use]
pub fn now_micros() -> u64 {
    cdk::utils::time::now_micros()
}

/// Return the current UNIX epoch time in nanoseconds.
#[must_use]
pub fn now_nanos() -> u64 {
    cdk::utils::time::now_nanos()
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
