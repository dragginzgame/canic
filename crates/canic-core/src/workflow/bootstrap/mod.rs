//! Bootstrap workflows.
//!
//! This module contains **async orchestration logic only**.
//! It assumes the environment has already been initialized or restored
//! by lifecycle adapters.
//!
//! It must NOT:
//! - handle IC lifecycle hooks directly
//! - depend on init payload presence
//! - perform environment seeding or restoration
//! - import directory snapshots outside explicit bootstrap rebuilds

pub mod nonroot;
pub mod root;

// Token used to restrict readiness transitions to bootstrap only.
pub struct ReadyToken(());

const fn ready_token() -> ReadyToken {
    ReadyToken(())
}
