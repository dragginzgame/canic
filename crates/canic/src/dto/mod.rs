pub use canic_core::dto::*;

#[cfg(feature = "control-plane")]
pub mod template {
    pub use canic_control_plane::dto::template::*;
}
