//
// Page
//
// Pagination envelope.
//

use crate::dto::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Page<T> {
    pub entries: Vec<T>,
    pub total: u64,
}

//
// PageRequest
//
// Pagination request.
//

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PageRequest {
    pub limit: u64,
    pub offset: u64,
}

#[cfg(all(test, feature = "wire-schema", not(target_arch = "wasm32")))]
mod tests;
