pub use canic_core::dto::*;

#[cfg(feature = "fleet-coordinator-canister")]
pub mod fleet_coordinator {
    pub use canic_control_plane::dto::fleet_coordinator::*;
}

#[cfg(any(feature = "control-plane", feature = "wasm-store-canister"))]
pub mod template {
    pub use canic_control_plane::dto::template::*;
}
