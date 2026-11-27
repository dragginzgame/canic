//! Root for controller-approved child creation and application readiness.
//! Root appends permanent services under the fleet subnet ceiling. Placement has eight shared-shard
//! identities and eight dedicated-instance identities under the approved subnet ceiling.
//! Bounded serial child upgrades and journal-preserving maintenance; no reset or arbitrary adoption.
//! Root admits per-receiver grants explicitly and records first use across upgrades.

mod auth;

mod canister;
