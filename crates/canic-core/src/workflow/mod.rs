//! Multi-step coordination and lifecycle orchestration.
//!
//! `workflow` sequences ops calls, schedules async follow-up work, and owns
//! behavior that unfolds over time.

pub mod auth;
#[cfg(feature = "blob-storage-billing")]
pub mod blob_storage;
pub mod bootstrap;
pub mod cascade;
pub mod component_runtime;
pub mod config;
pub mod cost_guard;
pub mod env;
pub mod fleet_admission_projection;
pub mod ic;
pub mod icrc;
pub mod log;
pub mod memory;
pub mod metrics;
pub mod placement;
pub mod replay;
pub mod rpc;
pub mod runtime;
pub mod state;
pub mod topology;
pub mod view;
