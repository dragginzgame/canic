//! Embedded asset-catalog witness using the real Blob service and Canic adapter.
//! This qualification application has no provider transport or settlement logic.
#[cfg(target_arch = "wasm32")]
mod canister;
pub mod protocol;
