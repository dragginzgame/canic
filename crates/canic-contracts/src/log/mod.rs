//! Shared log severity value; logging and rendering remain runtime-owned.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum Level {
    Debug,
    Info,
    Ok,
    Warn,
    Error,
}
