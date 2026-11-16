//! Experimental Root for up to two greeting applications, with controller-approved bounded creation.
//! Bounded serial child upgrades and journal-preserving maintenance; no reset or arbitrary adoption.
//! Optional `auth` enables one explicitly approved volatile issuer grant; upgrade closes it.
#[cfg(target_arch = "wasm32")]
mod canister;

#[cfg(all(target_arch = "wasm32", feature = "auth"))]
mod auth;
