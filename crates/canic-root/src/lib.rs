//! Root for controller-approved child creation and application readiness.
//! Fleet bootstrap bounds permanent service slots. Placement has eight shared-shard
//! identities and eight dedicated-instance identities under the approved subnet ceiling.
//! Bounded serial child upgrades and journal-preserving maintenance; no reset or arbitrary adoption.
//! Optional `auth` enables explicitly approved per-receiver volatile grants; upgrade closes it.

#[cfg(feature = "auth")]
mod auth;

mod canister;
