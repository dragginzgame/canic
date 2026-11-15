//! Versioned artifact custody. No child management, timers, deletion or deployment authority.
#[cfg(target_arch = "wasm32")]
mod canister;
#[cfg(any(target_arch = "wasm32", test))]
mod state;
