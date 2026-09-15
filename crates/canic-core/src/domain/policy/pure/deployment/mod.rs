//! Module: domain::policy::pure::deployment
//!
//! Responsibility: name the native reserve required by control-plane deployment admission.
//! Does not own: balances, reservations, spending or runtime effects.
//! Boundary: runtime admission and host startup planning consume the same floor.

/// Native cycles that must remain after a deployment call reservation.
/// This is the maintained runtime guard, independent of automatic top-up thresholds.
pub const MINIMUM_DEPLOYMENT_RESERVE_CYCLES: u128 = 1_000_000_000_000;
