//! Public control-plane APIs for lifecycle, state, and template publication.

#[cfg(feature = "root-control-plane")]
pub mod lifecycle;
#[cfg(feature = "root-control-plane")]
pub mod state;
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
pub mod template;
