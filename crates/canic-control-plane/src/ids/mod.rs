//! Control-plane-specific identifiers and value types.

use candid::{
    CandidType,
    types::{Serializer, Type},
};
pub use canic_core::ids::{BuildNetwork, CanisterRole};
use canic_memory::impl_storable_bounded;
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, borrow::Cow, fmt};

///
/// TemplateId
///

#[derive(
    CandidType, Clone, Debug, Eq, Ord, PartialOrd, Deserialize, Serialize, PartialEq, Hash,
)]
#[serde(transparent)]
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

impl_storable_bounded!(TemplateId, 160, false);

///
/// TemplateVersion
///

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
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

impl CandidType for TemplateVersion {
    fn _ty() -> Type {
        String::ty()
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: Serializer,
    {
        self.as_str().idl_serialize(serializer)
    }
}

impl_storable_bounded!(TemplateVersion, 64, false);

///
/// WasmStoreBinding
///

#[derive(
    CandidType, Clone, Debug, Eq, Ord, PartialOrd, Deserialize, Serialize, PartialEq, Hash,
)]
#[serde(transparent)]
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

impl_storable_bounded!(WasmStoreBinding, 64, false);

///
/// TemplateReleaseKey
///

#[derive(
    CandidType, Clone, Debug, Eq, Ord, PartialOrd, Deserialize, Serialize, PartialEq, Hash,
)]
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

impl_storable_bounded!(TemplateReleaseKey, 256, false);

///
/// TemplateChunkKey
///

#[derive(
    CandidType, Clone, Debug, Eq, Ord, PartialOrd, Deserialize, Serialize, PartialEq, Hash,
)]
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

impl_storable_bounded!(TemplateChunkKey, 320, false);

#[cfg(test)]
mod tests {
    use super::{CanisterRole, TemplateVersion};
    use candid::{CandidType, Encode};

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
    fn template_version_uses_string_candid_encoding() {
        let version = TemplateVersion::new("0.18.5");

        assert_eq!(TemplateVersion::ty(), String::ty());
        assert_eq!(Encode!(&version).unwrap(), Encode!(&"0.18.5").unwrap());
    }
}

///
/// TemplateChunkingMode
///

#[derive(
    CandidType, Clone, Copy, Debug, Eq, Ord, PartialOrd, Deserialize, Serialize, PartialEq,
)]
pub enum TemplateChunkingMode {
    Inline,
    Chunked,
}

///
/// WasmStoreGcMode
///

#[derive(
    CandidType, Clone, Copy, Debug, Default, Eq, Ord, PartialOrd, Deserialize, Serialize, PartialEq,
)]
pub enum WasmStoreGcMode {
    #[default]
    Normal,
    Prepared,
    InProgress,
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

#[derive(
    CandidType, Clone, Copy, Debug, Eq, Ord, PartialOrd, Deserialize, Serialize, PartialEq,
)]
pub enum TemplateManifestState {
    Staged,
    Approved,
    Blocked,
    Deprecated,
}
