//! Module: model::cycles_funding
//!
//! Responsibility: own authoritative cycles-funding limits and ledger values.
//! Does not own: funding decisions, stable record conversion, or grant execution.

///
/// FundingLimits
///
/// Effective parent funding limits for a single child role.
///

#[derive(Clone, Copy, Debug)]
pub struct FundingLimits {
    pub max_per_request: u128,
    pub max_per_child: u128,
    pub cooldown_secs: u64,
}

///
/// FundingLedgerSnapshot
///
/// Read-only child funding ledger state used by policy evaluation.
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FundingLedgerSnapshot {
    pub granted_total: u128,
    pub last_granted_at: u64,
}
