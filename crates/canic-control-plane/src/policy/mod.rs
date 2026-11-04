//! Pure control-plane decisions, independent of state access and effects.
pub mod continuation_retry;
#[cfg(feature = "fleet-coordinator-canister")]
pub mod funding_acceptance;
