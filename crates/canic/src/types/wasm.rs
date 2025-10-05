//!
//! Lightweight wrapper for statically embedded WASM modules, providing helper
//! methods for hashing and metadata used by registry components.
//!

use crate::utils::wasm::get_wasm_hash;

///
/// WasmModule
/// Holds a reference to embedded WASM bytes and exposes helper inspectors.
///

#[derive(Clone, Debug)]
pub struct WasmModule {
    bytes: &'static [u8],
}

impl WasmModule {
    #[must_use]
    pub const fn new(bytes: &'static [u8]) -> Self {
        Self { bytes }
    }

    #[must_use]
    pub fn module_hash(&self) -> Vec<u8> {
        get_wasm_hash(self.bytes)
    }

    #[must_use]
    pub const fn bytes(&self) -> &'static [u8] {
        self.bytes
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.bytes.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}
