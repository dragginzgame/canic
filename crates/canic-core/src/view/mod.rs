//! Module: view
//!
//! Responsibility: group internal read-only projections over stored or runtime state.
//! Does not own: endpoint DTOs, stable records, or workflow decisions.
//! Boundary: ops and workflow use views internally before endpoint DTO shaping.

pub mod authority_restore;
pub mod fleet_activation;
pub mod icp_refill;
pub mod intent;
pub mod intent_release;
pub mod provisioning_failure;
pub mod public_metrics;
pub mod replay_release;
pub mod state_cascade;
