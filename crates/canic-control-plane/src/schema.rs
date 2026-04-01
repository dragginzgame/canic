use crate::ids::CanisterRole;
use serde::{Deserialize, Serialize};

const IMPLICIT_WASM_STORE_ROLE: CanisterRole = CanisterRole::WASM_STORE;
const IMPLICIT_WASM_STORE_HEADROOM_BYTES: u64 = 4_000_000;

///
/// WasmStoreConfig
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WasmStoreConfig {
    pub canister_role: CanisterRole,

    #[serde(default)]
    pub policy: WasmStorePolicy,
}

impl WasmStoreConfig {
    /// Build the one implicit wasm-store preset used on every subnet.
    #[must_use]
    pub fn implicit() -> Self {
        Self {
            canister_role: IMPLICIT_WASM_STORE_ROLE,
            policy: WasmStorePolicy {
                max_store_bytes: implicit_wasm_store_max_store_bytes(),
                headroom_bytes: Some(IMPLICIT_WASM_STORE_HEADROOM_BYTES),
                max_templates: None,
                max_template_versions_per_template: None,
            },
        }
    }

    /// Return the configured hard occupied-byte ceiling for this store.
    #[must_use]
    pub const fn max_store_bytes(&self) -> u64 {
        self.policy.max_store_bytes
    }

    /// Return the configured logical template ceiling for this store, if any.
    #[must_use]
    pub const fn max_templates(&self) -> Option<u32> {
        self.policy.max_templates
    }

    /// Return the configured reserve headroom threshold for this store, if any.
    #[must_use]
    pub const fn headroom_bytes(&self) -> Option<u64> {
        self.policy.headroom_bytes
    }

    /// Return the configured retained-version ceiling per template, if any.
    #[must_use]
    pub const fn max_template_versions_per_template(&self) -> Option<u16> {
        self.policy.max_template_versions_per_template
    }
}

// Resolve the implicit store capacity selected at compile time for this build.
fn implicit_wasm_store_max_store_bytes() -> u64 {
    env!("CANIC_IMPLICIT_WASM_STORE_MAX_STORE_BYTES")
        .parse::<u64>()
        .expect("CANIC_IMPLICIT_WASM_STORE_MAX_STORE_BYTES must be a valid u64")
}

///
/// WasmStorePolicy
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WasmStorePolicy {
    pub max_store_bytes: u64,

    #[serde(default)]
    pub headroom_bytes: Option<u64>,

    #[serde(default)]
    pub max_templates: Option<u32>,

    #[serde(default)]
    pub max_template_versions_per_template: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::WasmStoreConfig;
    use crate::ids::CanisterRole;

    #[test]
    fn wasm_store_policy_is_the_implicit_ic_preset() {
        let store = WasmStoreConfig::implicit();

        assert_eq!(store.canister_role, CanisterRole::WASM_STORE);
        assert_eq!(store.max_store_bytes(), 40_000_000);
        assert_eq!(store.headroom_bytes(), Some(4_000_000));
        assert_eq!(store.max_templates(), None);
        assert_eq!(store.max_template_versions_per_template(), None);
    }
}
