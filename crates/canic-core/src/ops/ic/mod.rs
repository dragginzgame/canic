//! Ops-level façade for Internet Computer primitives.
//!
//! This module defines the **approved IC surface area** for the rest of the system.
//! It deliberately sits between higher layers (workflow, endpoints, macros) and
//! the low-level IC infrastructure implementations in `infra::ic`.
//!
//! Design intent:
//! - `infra::ic` owns **raw, minimal IC bindings**
//!   (management canister calls, signatures, timers, randomness, etc.).
//! - `ops::ic` owns the **public contract** exposed to application code.
//!
//! As a result, most items here are **re-exports** rather than reimplementations.
//! This is intentional:
//! - It prevents higher layers from depending on `infra` directly.
//! - It allows `infra` to be refactored or replaced without touching call sites.
//! - It centralizes IC usage under a single, stable import path.
//!
//! Wrapping vs pass-through:
//! - Items are **wrapped** here when ops-level concerns apply
//!   (metrics, logging, perf tracking, lifecycle conventions).
//! - Items are **passed through** unchanged when the raw IC API is already
//!   the desired abstraction and no additional policy or orchestration is needed.
//!
//! In other words:
//! - `infra::ic` answers “how does the IC work?”
//! - `ops::ic` answers “what IC functionality is allowed to be used?”
//!
//! This module intentionally contains no policy decisions and no workflow logic.

pub mod call;
pub mod http;
pub mod mgmt;
pub mod nns;
pub mod signature;
pub mod xrc;

use crate::{
    Error, ThisError,
    cdk::{
        call::{CallFailed, CandidDecodeFailed},
        candid::Error as CandidError,
    },
    ops::OpsError,
};

///
/// IcOpsError
///

#[derive(Debug, ThisError)]
pub enum IcOpsError {
    #[error(transparent)]
    HttpOps(#[from] http::HttpOpsError),

    #[error(transparent)]
    XrcOps(#[from] xrc::XrcOpsError),

    #[error("ic call failed: {0}")]
    CallFailed(#[from] CallFailed),

    #[error("candid error: {0}")]
    Candid(#[from] CandidError),

    #[error("candid decode failed: {0}")]
    CandidDecodeFailed(#[from] CandidDecodeFailed),
}

impl From<IcOpsError> for Error {
    fn from(err: IcOpsError) -> Self {
        OpsError::from(err).into()
    }
}
