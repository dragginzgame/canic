//! Experimental Root for up to two applications with explicit readiness and approved creation.
//! Bounded serial child upgrades and journal-preserving maintenance; no reset or arbitrary adoption.
//! Optional `auth` enables one explicitly approved volatile issuer grant; upgrade closes it.
#[cfg(all(target_arch = "wasm32", feature = "backup"))]
mod backup;
#[cfg(target_arch = "wasm32")]
mod canister;

#[cfg(all(target_arch = "wasm32", feature = "auth"))]
mod auth;
