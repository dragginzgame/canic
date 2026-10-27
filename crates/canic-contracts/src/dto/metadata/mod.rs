//
// CanicMetadataResponse
//

use crate::dto::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CanicMetadataResponse {
    pub package_name: String,
    pub package_version: String,
    pub package_description: String,
    pub canic_version: String,
    pub canister_version: u64,
}
