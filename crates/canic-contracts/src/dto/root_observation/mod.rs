//! Passive native envelope for one exact Root-routed observation.
//!
//! The method selects the payload type; no capability-dependent union is encoded.

/// Target and operation input; Root and the target retain their independent admission checks.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Request<T> {
    pub canister_id: ic_platform::Principal,
    pub input: T,
}
