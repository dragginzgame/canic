//! Experimental Root for up to two greeting applications, with controller-approved bounded creation.
//! Bounded serial child upgrades and journal-preserving maintenance; no reset or arbitrary adoption.
#[cfg(target_arch = "wasm32")]
mod canister;
