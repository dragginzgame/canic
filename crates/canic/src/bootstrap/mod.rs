// -----------------------------------------------------------------------------
// Embedded bootstrap artifacts
// -----------------------------------------------------------------------------

// Return the canonical built-in bootstrap wasm for the first live `wasm_store`.
#[must_use]
pub const fn root_wasm_store_wasm() -> &'static [u8] {
    include_bytes!("../../assets/bootstrap/wasm_store.wasm.gz")
}
