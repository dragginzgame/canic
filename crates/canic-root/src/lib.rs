//! Experimental Root for up to two applications with explicit readiness and approved creation.
//! Bounded serial child upgrades and journal-preserving maintenance; no reset or arbitrary adoption.
//! Optional `auth` enables explicitly approved per-receiver volatile grants; upgrade closes it.

#[cfg(feature = "auth")]
mod auth;

// Management currently needs the IC timer executor and raw cycle/stable-memory observations.
#[cfg(target_arch = "wasm32")]
mod canister;
