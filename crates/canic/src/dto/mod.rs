pub use canic_core::dto::*;

#[cfg(any(feature = "control-plane", feature = "wasm-store-canister"))]
pub mod template {
    pub use canic_control_plane::dto::template::*;
}
