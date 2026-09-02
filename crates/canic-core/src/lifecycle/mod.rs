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
    ic_cdk::api::trap(format!("{phase}: {err}"))
}

fn retryable_nonroot_bootstrap_error(error: &crate::InternalError) -> bool {
    let code = error.public_error().code();
    [
        crate::diagnostics::codes::LIFECYCLE_INACTIVE,
        crate::diagnostics::codes::LIFECYCLE_UNAVAILABLE,
        crate::diagnostics::codes::PLATFORM_FAILED,
        crate::diagnostics::codes::PLATFORM_UNAVAILABLE,
        crate::diagnostics::codes::CAPACITY_INSUFFICIENT,
        crate::diagnostics::codes::STATE_UNAVAILABLE,
    ]
    .into_iter()
    .any(|retryable| code == retryable.raw_code())
}

#[cfg(test)]
mod tests {
    use super::retryable_nonroot_bootstrap_error;

    #[test]
    fn nonroot_bootstrap_retries_transient_pool_shortage_but_not_hard_capacity() {
        assert!(retryable_nonroot_bootstrap_error(
            &crate::InternalError::public(crate::diagnostics::codes::CAPACITY_INSUFFICIENT)
        ));
        assert!(!retryable_nonroot_bootstrap_error(
            &crate::InternalError::resource_exhausted()
        ));
    }
}
