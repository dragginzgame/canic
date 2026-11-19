//! Experimental Root for up to two applications with explicit readiness and approved creation.
//! Bounded serial child upgrades and journal-preserving maintenance; no reset or arbitrary adoption.
//! Optional `auth` enables explicitly approved per-receiver volatile grants; upgrade closes it.
#![cfg(target_arch = "wasm32")]

#[cfg(feature = "backup")]
mod backup;
mod canister;

#[cfg(feature = "auth")]
mod auth;

#[cfg(feature = "sharding")]
mod sharding;
