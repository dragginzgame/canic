//! Internal test scheduling choices, independent of durable grant and chunk state.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum ReplyKind {
    Grant,
    Revoke,
    Chunk,
}
