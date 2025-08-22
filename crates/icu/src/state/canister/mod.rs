mod canister_catalog;

pub use canister_catalog::*;

use crate::utils::wasm::get_wasm_hash;

///
/// CanisterConfig
///

#[derive(Clone, Debug)]
pub struct CanisterConfig {
    pub attributes: CanisterAttributes,
    pub wasm: &'static [u8],
}

impl CanisterConfig {
    #[must_use]
    pub fn module_hash(&self) -> Vec<u8> {
        get_wasm_hash(self.wasm)
    }
}

///
/// CanisterConfigView
/// the front-facing version
///

#[derive(Clone, Debug)]
pub struct CanisterConfigView {
    pub attributes: CanisterAttributes,
    pub wasm_size: usize,
}

impl From<&CanisterConfig> for CanisterConfigView {
    fn from(canister: &CanisterConfig) -> Self {
        Self {
            attributes: canister.attributes.clone(),
            wasm_size: canister.wasm.len(),
        }
    }
}

///
/// CanisterAttributes
///
/// auto_create : number of canisters to create on root
///

#[derive(Clone, Debug, Default)]
pub struct CanisterAttributes {
    pub auto_create: Option<u16>,
    pub uses_directory: bool,
}
