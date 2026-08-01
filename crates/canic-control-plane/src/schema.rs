const IMPLICIT_WASM_STORE_MAX_STORE_BYTES: u64 = 40_000_000;
const IMPLICIT_WASM_STORE_HEADROOM_BYTES: u64 = 4_000_000;

///
/// WasmStoreConfig
///

#[derive(Clone, Copy, Debug)]
pub struct WasmStoreConfig {
    max_store_bytes: u64,
    headroom_bytes: Option<u64>,
    max_templates: Option<u32>,
    max_template_versions_per_template: Option<u16>,
}

impl WasmStoreConfig {
    /// Build the one implicit wasm-store preset used on every subnet.
    #[must_use]
    pub const fn implicit() -> Self {
        Self {
            max_store_bytes: IMPLICIT_WASM_STORE_MAX_STORE_BYTES,
            headroom_bytes: Some(IMPLICIT_WASM_STORE_HEADROOM_BYTES),
            max_templates: None,
            max_template_versions_per_template: None,
        }
    }

    /// Return the configured hard occupied-byte ceiling for this store.
    #[must_use]
    pub const fn max_store_bytes(&self) -> u64 {
        self.max_store_bytes
    }

    /// Return the configured logical template ceiling for this store, if any.
    #[must_use]
    pub const fn max_templates(&self) -> Option<u32> {
        self.max_templates
    }

    /// Return the configured reserve headroom threshold for this store, if any.
    #[must_use]
    pub const fn headroom_bytes(&self) -> Option<u64> {
        self.headroom_bytes
    }

    /// Return the configured retained-version ceiling per template, if any.
    #[must_use]
    pub const fn max_template_versions_per_template(&self) -> Option<u16> {
        self.max_template_versions_per_template
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::WasmStoreConfig;

    #[test]
    fn wasm_store_policy_is_the_implicit_ic_preset() {
        let store = WasmStoreConfig::implicit();

        assert_eq!(store.max_store_bytes(), 40_000_000);
        assert_eq!(store.headroom_bytes(), Some(4_000_000));
        assert_eq!(store.max_templates(), None);
        assert_eq!(store.max_template_versions_per_template(), None);
    }
}
