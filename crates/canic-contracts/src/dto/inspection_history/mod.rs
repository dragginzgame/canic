//! Exact native target for replicated management history.
use serde::{Deserialize, Serialize};
/// Controller-selected management history target.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Request {
    pub canister_id: ic_platform::Principal,
}
