//! Application-owned discovery; worker storage and caller authentication stay at the worker.
#[cfg(target_arch = "wasm32")]
mod canister;
