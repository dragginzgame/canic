//! Ops layer: approved execution surface and coordination boundary.
//!
//! The `ops` layer defines the **sanctioned capabilities** that higher layers
//! (workflow, API, macros) are allowed to use. It sits between application logic
//! and low-level infrastructure, providing a stable execution façade.
//!
//! Responsibilities:
//! - Expose approved primitives and subsystems (IC access, runtime context,
//!   metrics, logging, registries).
//! - Add cross-cutting concerns such as metrics, logging, and normalization.
//! - Aggregate infra errors into ops-scoped error types.
//!
//! Non-responsibilities:
//! - No business policy or workflow orchestration.
//! - No domain decisions or lifecycle management.
//!
//! Infra interaction:
//! - `infra` owns **raw mechanical implementations** (IC calls, encoding,
//!   decoding, management canister interactions).
//! - `ops` may either wrap infra or call the CDK directly when the CDK API
//!   already represents the desired primitive (e.g. ambient runtime context).
//!
//! Naming conventions:
//! - Plain nouns (e.g. `Call`, `Runtime`, `Env`) represent approved execution
//!   primitives.
//! - `*Ops` types represent orchestration or aggregation roles (typically error
//!   or coordination objects), not primitives themselves.

pub mod call;
pub mod http;
pub mod ledger;
pub mod mgmt;
pub mod network;
pub mod nns;
pub mod signature;
pub mod xrc;

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

#[derive(Debug, ThisError)]
pub enum IcOpsError {
    #[error(transparent)]
    Infra(#[from] infra::InfraError),

    #[error(transparent)]
    CallOps(#[from] call::CallError),

    #[error(transparent)]
    HttpOps(#[from] http::HttpOpsError),

    #[error(transparent)]
    LedgerOps(#[from] ledger::LedgerOpsError),

    #[error(transparent)]
    XrcOps(#[from] xrc::XrcOpsError),
}

impl From<IcOpsError> for InternalError {
    fn from(err: IcOpsError) -> Self {
        OpsError::from(err).into()
    }
}

///
/// IcOps
/// Ambient IC execution primitives
///

pub struct IcOps;

impl IcOps {
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

    /// Return the current UNIX epoch time in seconds.
    #[must_use]
    pub fn now_secs() -> u64 {
        cdk::utils::time::now_secs()
    }

    /// Return the current UNIX epoch time in milliseconds.
    #[must_use]
    #[expect(dead_code)]
    pub fn now_millis() -> u64 {
        cdk::utils::time::now_millis()
    }

    /// Return the current UNIX epoch time in microseconds.
    #[must_use]
    #[expect(dead_code)]
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
}
