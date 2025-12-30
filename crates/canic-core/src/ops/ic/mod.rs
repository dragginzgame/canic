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
pub mod mgmt;

pub use crate::infra::ic::{Network, build_network, build_network_from_dfx_network};
pub use call::Call;
pub use mgmt::{
    call_and_decode, canister_cycle_balance, canister_status, create_canister, delete_canister,
    deposit_cycles, get_cycles, install_code, raw_rand, uninstall_code, update_settings,
    upgrade_canister,
};

pub mod signature {
    pub use crate::infra::ic::signature::*;
}

pub mod timer {
    pub use crate::ops::runtime::timer::{TimerId, TimerOps};
}
