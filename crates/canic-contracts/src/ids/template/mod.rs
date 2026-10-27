//! Control-plane-specific identifiers and value types.

use serde::{Deserialize, Serialize};
use std::{
    borrow::{Borrow, Cow},
    fmt,
};

///
/// TemplateId
///

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TemplateId(pub Cow<'static, str>);

impl TemplateId {
    #[must_use]
    pub const fn new(s: &'static str) -> Self {
        Self(Cow::Borrowed(s))
    }

    #[must_use]
    pub const fn owned(s: String) -> Self {
        Self(Cow::Owned(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for TemplateId {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

impl From<String> for TemplateId {
    fn from(value: String) -> Self {
        Self::owned(value)
    }
}

impl From<&String> for TemplateId {
    fn from(value: &String) -> Self {
        Self::owned(value.clone())
    }
}

impl Borrow<str> for TemplateId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for TemplateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

///
/// TemplateVersion
///

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TemplateVersion(pub Cow<'static, str>);

impl TemplateVersion {
    #[must_use]
    pub const fn new(s: &'static str) -> Self {
        Self(Cow::Borrowed(s))
    }

    #[must_use]
    pub const fn owned(s: String) -> Self {
        Self(Cow::Owned(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for TemplateVersion {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

impl From<String> for TemplateVersion {
    fn from(value: String) -> Self {
        Self::owned(value)
    }
}

impl From<&String> for TemplateVersion {
    fn from(value: &String) -> Self {
        Self::owned(value.clone())
    }
}

impl Borrow<str> for TemplateVersion {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for TemplateVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

///
/// WasmStoreBinding
///

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct WasmStoreBinding(pub Cow<'static, str>);

impl WasmStoreBinding {
    #[must_use]
    pub const fn new(s: &'static str) -> Self {
        Self(Cow::Borrowed(s))
    }

    #[must_use]
    pub const fn owned(s: String) -> Self {
        Self(Cow::Owned(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for WasmStoreBinding {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WasmStoreBinding {
    fn from(value: String) -> Self {
        Self::owned(value)
    }
}

impl From<&String> for WasmStoreBinding {
    fn from(value: &String) -> Self {
        Self::owned(value.clone())
    }
}

impl Borrow<str> for WasmStoreBinding {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for WasmStoreBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

///
/// TemplateReleaseKey
///

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct TemplateReleaseKey {
    pub template_id: TemplateId,
    pub version: TemplateVersion,
}

impl TemplateReleaseKey {
    #[must_use]
    pub const fn new(template_id: TemplateId, version: TemplateVersion) -> Self {
        Self {
            template_id,
            version,
        }
    }
}

impl fmt::Display for TemplateReleaseKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.template_id, self.version)
    }
}

///
/// TemplateChunkKey
///

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct TemplateChunkKey {
    pub release: TemplateReleaseKey,
    pub chunk_index: u32,
}

impl TemplateChunkKey {
    #[must_use]
    pub const fn new(release: TemplateReleaseKey, chunk_index: u32) -> Self {
        Self {
            release,
            chunk_index,
        }
    }
}

impl fmt::Display for TemplateChunkKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.release, self.chunk_index)
    }
}

#[cfg(test)]
mod tests {
    use super::TemplateVersion;
    use crate::ids::CanisterRole;

    #[cfg(feature = "wire-schema")]
    #[test]
    fn template_identifiers_project_and_encode_their_actual_text() {
        use super::TemplateId;
        let limits = crate::protocol::public_health::RESPONSE_LIMITS;
        let method = crate::protocol::public_health::METHOD_ID;
        for text in ["", "template", "é雪"] {
            let borrowed = TemplateId::new(text);
            let owned = TemplateId::owned(text.to_owned());
            let version = TemplateVersion::owned(text.to_owned());
            let bytes = ic_wire::native::encode_request(method, &text, limits).unwrap();
            assert_eq!(
                ic_wire::native::encode_request(method, &borrowed, limits).unwrap(),
                bytes
            );
            assert_eq!(
                ic_wire::native::encode_request(method, &owned, limits).unwrap(),
                bytes
            );
            assert_eq!(
                ic_wire::native::encode_request(method, &version, limits).unwrap(),
                bytes
            );
            assert_eq!(
                ic_wire::native::decode_request::<TemplateId>(method, &bytes, limits).unwrap(),
                owned
            );
            assert_eq!(
                ic_wire::native::decode_request::<TemplateVersion>(method, &bytes, limits).unwrap(),
                version
            );
        }
        let text = ic_wire::typescript::native_schema::<String>().unwrap();
        assert_eq!(
            ic_wire::typescript::native_schema::<TemplateId>().unwrap(),
            text
        );
        assert_eq!(
            ic_wire::typescript::native_schema::<TemplateVersion>().unwrap(),
            text
        );
    }

    #[test]
    fn canister_role_basic_traits_and_utils() {
        let a = CanisterRole::ROOT;
        assert!(a.is_root());
        assert_eq!(a.as_str(), "root");
        let b: CanisterRole = "example".into();
        assert_eq!(b.as_str(), "example");
        let s: String = b.clone().into();
        assert_eq!(s, "example");
        assert_eq!(b.as_ref(), "example");
    }

    #[test]
    fn template_version_uses_string_wire_encoding() {
        let version = TemplateVersion::new("0.18.5");

        assert_eq!(
            crate::protocol::authority::encode(&version).unwrap(),
            crate::protocol::authority::encode(&"0.18.5").unwrap()
        );
    }
}

///
/// TemplateChunkingMode
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum TemplateChunkingMode {
    Chunked,
}

///
/// WasmStoreGcMode
///

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum WasmStoreGcMode {
    #[default]
    Normal,
    Prepared,
    InProgress,
    Clearing,
    Complete,
}

///
/// WasmStoreGcStatus
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WasmStoreGcStatus {
    pub mode: WasmStoreGcMode,
    pub changed_at: u64,
    pub prepared_at: Option<u64>,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub runs_completed: u32,
}

///
/// TemplateManifestState
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum TemplateManifestState {
    Approved,
}
