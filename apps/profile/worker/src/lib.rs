//! Standalone application storage and lifecycle; Canic owns deployment only.
#[cfg(target_arch = "wasm32")]
mod canister;
#[cfg(any(target_arch = "wasm32", test))]
mod state;
