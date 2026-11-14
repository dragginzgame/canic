//! Experimental Root for one greeting application, with controller-approved bounded creation.
//! Bounded serial child upgrades and journal-preserving maintenance; no reset or arbitrary adoption.
#[cfg(target_arch = "wasm32")]
mod canister;
