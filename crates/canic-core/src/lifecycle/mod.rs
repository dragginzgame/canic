//! Lifecycle adapters.
//!
//! This module is public solely so it can be referenced by
//! macro expansions in downstream crates. It is not intended
//! for direct use.
//!
//! It must remain synchronous and minimal.

pub mod init;
pub mod upgrade;

use std::fmt;

///
/// LifecyclePhase
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecyclePhase {
    Init,
    PostUpgrade,
}

impl fmt::Display for LifecyclePhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Init => f.write_str("init"),
            Self::PostUpgrade => f.write_str("post_upgrade"),
        }
    }
}

pub fn lifecycle_trap(phase: LifecyclePhase, err: impl fmt::Display) -> ! {
    crate::cdk::api::trap(format!("{phase}: {err}"))
}
