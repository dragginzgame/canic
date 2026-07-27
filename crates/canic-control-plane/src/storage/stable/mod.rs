#[cfg(any(
    feature = "fleet-coordinator-canister",
    feature = "root-control-plane",
    feature = "wasm-store-canister"
))]
pub mod fleet_coordinator;
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
pub mod state;
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
pub mod template;
