//! Passive protocol classifications; runtime decisions remain with Core.

use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize)]
#[remain::sorted]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum MetricsKind {
    Core,
    Placement,
    Platform,
    Runtime,
    Security,
    Storage,
}
