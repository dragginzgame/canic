//! Module: ops::ic
//!
//! Responsibility: expose approved IC runtime and platform-call operations.
//! Does not own: business policy, workflow orchestration, or lifecycle decisions.
//! Boundary: ops layer between workflows and raw infra/CDK IC primitives.

pub mod build_network;
pub mod call;
pub mod http;
pub mod icp_refill;
pub mod ledger;
pub mod mgmt;
pub mod nns;

use crate::{
    InternalError,
    cdk::{self, types::Principal},
    infra,
    ops::OpsError,
};
use thiserror::Error as ThisError;

///
/// IcOpsError
///
/// Typed failure surface for IC operation facades.
///

#[derive(Debug, ThisError)]
pub enum IcOpsError {
    #[error(transparent)]
    Infra(#[from] infra::InfraError),

    #[error(transparent)]
    CallOps(#[from] call::CallError),

    #[error(transparent)]
    HttpOps(#[from] http::HttpOpsError),

    #[error(transparent)]
    IcpRefillOps(#[from] icp_refill::IcpRefillOpsError),

    #[error(transparent)]
    LedgerOps(#[from] ledger::LedgerOpsError),
}

impl From<IcOpsError> for InternalError {
    fn from(err: IcOpsError) -> Self {
        OpsError::from(err).into()
    }
}

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
        cdk::api::canister_self()
    }

    /// Return the current canister's cycle balance.
    #[must_use]
    pub fn canister_cycle_balance() -> crate::cdk::types::Cycles {
        cdk::api::canister_cycle_balance().into()
    }

    /// Return the current caller principal.
    #[must_use]
    pub fn msg_caller() -> Principal {
        cdk::api::msg_caller()
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
}
