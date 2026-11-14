//! Experimental Root for one greeting application, with controller-approved bounded creation.
//! No reset, arbitrary child adoption, child upgrades or subtree operations are exposed.
#[cfg(target_arch = "wasm32")]
mod canister;
