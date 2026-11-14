//! Experimental Root for one greeting application, with controller-approved bounded creation.
//! One retained child upgrade is supported; no reset, arbitrary adoption or subtree operations.
#[cfg(target_arch = "wasm32")]
mod canister;
