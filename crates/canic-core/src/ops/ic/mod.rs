//! Module: ops::ic
//!
//! Responsibility: expose approved IC runtime and platform-call operations.
//! Does not own: business policy, workflow orchestration, or lifecycle decisions.
//! Boundary: ops layer between workflows and raw infra/CDK IC primitives.

pub mod build_network;
pub mod call;
pub mod icp_refill;
pub mod mgmt;
pub mod nns;
pub mod release_build;

use crate::{
    InternalError,
    cdk::types::{Cycles, Principal},
};
use std::time::SystemTime;

///
/// IcOps
///
/// Operations-layer facade for ambient IC execution primitives.
///

pub struct IcOps;

impl IcOps {
    /// Return the current canister principal.
    #[must_use]
    pub fn canister_self() -> Principal {
        ic_cdk::api::canister_self()
    }

    /// Return the current canister's cycle balance.
    #[must_use]
    pub fn canister_cycle_balance() -> crate::cdk::types::Cycles {
        ic_cdk::api::canister_cycle_balance().into()
    }

    /// Return the exact cycles that must accompany creation to leave the new Canister funded.
    pub fn canister_creation_attached_cycles(
        initial_cycles: &Cycles,
    ) -> Result<Cycles, InternalError> {
        checked_canister_creation_attached_cycles(
            initial_cycles.to_u128(),
            ic_cdk::api::cost_create_canister(),
        )
        .map(Cycles::new)
        .ok_or_else(|| {
            InternalError::resource_exhausted(
                "Canister creation funding plus the current Subnet creation cost exceeds u128",
            )
        })
    }

    /// Return the current caller principal.
    #[must_use]
    pub fn msg_caller() -> Principal {
        ic_cdk::api::msg_caller()
    }

    /// Return the physical Subnet currently executing this canister.
    #[must_use]
    pub fn subnet_self() -> Principal {
        ic_cdk::api::subnet_self()
    }

    /// Return a metadata-hash caller principal on both IC and host targets.
    #[must_use]
    #[cfg_attr(
        not(target_arch = "wasm32"),
        expect(
            clippy::missing_const_for_fn,
            reason = "wasm path delegates to ic0-backed caller lookup, which is not const"
        )
    )]
    pub(crate) fn metadata_entropy_caller() -> Principal {
        #[cfg(target_arch = "wasm32")]
        {
            Self::msg_caller()
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            Principal::anonymous()
        }
    }

    /// Return a metadata-hash canister principal on both IC and host targets.
    #[must_use]
    #[cfg_attr(
        not(target_arch = "wasm32"),
        expect(
            clippy::missing_const_for_fn,
            reason = "wasm path delegates to ic0-backed canister lookup, which is not const"
        )
    )]
    pub(crate) fn metadata_entropy_canister() -> Principal {
        #[cfg(target_arch = "wasm32")]
        {
            Self::canister_self()
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            Principal::management_canister()
        }
    }

    /// Return the current UNIX epoch time in seconds.
    #[must_use]
    #[expect(clippy::cast_possible_truncation)]
    pub fn now_secs() -> u64 {
        (time_nanos() / 1_000_000_000) as u64
    }

    /// Return the current UNIX epoch time in milliseconds.
    #[must_use]
    #[expect(clippy::cast_possible_truncation)]
    pub fn now_millis() -> u64 {
        (time_nanos() / 1_000_000) as u64
    }

    /// Return the current UNIX epoch time in nanoseconds.
    #[must_use]
    #[expect(clippy::cast_possible_truncation)]
    pub fn now_nanos() -> u64 {
        time_nanos() as u64
    }

    /// Print a line to the IC debug output.
    pub fn println(message: &str) {
        ic_cdk::println!("{message}");
    }

    /// Abort the current IC message so partially established runtime state is rolled back.
    pub fn trap(message: impl Into<String>) -> ! {
        ic_cdk::api::trap(message.into())
    }

    /// Spawn a task on the IC runtime.
    pub fn spawn<F>(future: F)
    where
        F: Future<Output = ()> + 'static,
    {
        ic_cdk::futures::spawn(future);
    }
}

const fn checked_canister_creation_attached_cycles(
    initial_cycles: u128,
    creation_cost: u128,
) -> Option<u128> {
    initial_cycles.checked_add(creation_cost)
}

/// Return the current UNIX epoch time in nanoseconds as the internal base unit.
#[cfg_attr(target_arch = "wasm32", expect(unreachable_code))]
fn time_nanos() -> u128 {
    #[cfg(target_arch = "wasm32")]
    {
        return u128::from(ic_cdk::api::time());
    }

    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos(),
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{IcOps, checked_canister_creation_attached_cycles};

    #[test]
    fn current_time_is_a_recent_unix_timestamp() {
        assert!(IcOps::now_secs() > 1_700_000_000);
    }

    #[test]
    fn canister_creation_funding_includes_platform_cost_without_overflow() {
        assert_eq!(
            checked_canister_creation_attached_cycles(1_000_000_000_000, 1_307_692_307_692),
            Some(2_307_692_307_692)
        );
        assert_eq!(
            checked_canister_creation_attached_cycles(u128::MAX, 1),
            None
        );
    }
}
