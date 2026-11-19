//! Root for controller-approved child creation and application readiness.
//! Ordinary fleet plans support two application slots. With `sharding`, Root can
//! allocate up to eight children across shared-shard and dedicated-project pools under one subnet ceiling.
//! Bounded serial child upgrades and journal-preserving maintenance; no reset or arbitrary adoption.
//! Optional `auth` enables explicitly approved per-receiver volatile grants; upgrade closes it.

#[cfg(feature = "auth")]
mod auth;

// Management currently needs the IC timer executor and raw cycle/stable-memory observations.
#[cfg(target_arch = "wasm32")]
mod canister;
