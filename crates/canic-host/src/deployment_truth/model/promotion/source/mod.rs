use super::super::VerifiedPostconditionV1;
use serde::{Deserialize, Serialize};

///
/// ArtifactTransportV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ArtifactTransportV1 {
    LocalCli,
    WasmStore,
    DirectAgent,
}

///
/// StagingReceiptV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StagingReceiptV1 {
    pub schema_version: u32,
    pub role: String,
    pub artifact_identity: String,
    pub transport: ArtifactTransportV1,
    pub wasm_store_locator: Option<String>,
    pub prepared_chunk_hashes: Vec<String>,
    pub published_chunk_count: usize,
    pub verified_postcondition: VerifiedPostconditionV1,
}

///
/// RoleArtifactSourceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleArtifactSourceV1 {
    pub role: String,
    pub kind: RoleArtifactSourceKindV1,
    pub locator: Option<String>,
    pub previous_receipt_kind: Option<PreviousArtifactReceiptKindV1>,
    pub previous_receipt_lineage_digest: Option<String>,
    pub expected_wasm_sha256: Option<String>,
    pub expected_wasm_gz_sha256: Option<String>,
    pub expected_candid_sha256: Option<String>,
    pub expected_canonical_embedded_config_sha256: Option<String>,
}

///
/// RolePromotionInputV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionInputV1 {
    pub role: String,
    pub promotion_level: PromotionArtifactLevelV1,
    pub source: RoleArtifactSourceV1,
    pub require_byte_identical_wasm: bool,
    pub require_target_embedded_config: bool,
    pub target_store_has_artifact: Option<bool>,
}

///
/// PromotionArtifactLevelV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PromotionArtifactLevelV1 {
    SealedWasm,
    SourceBuild,
}

///
/// PromotionReadinessStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PromotionReadinessStatusV1 {
    Ready,
    Blocked,
}

impl PromotionReadinessStatusV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Blocked => "blocked",
        }
    }
}

///
/// RoleArtifactSourceKindV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RoleArtifactSourceKindV1 {
    WorkspacePackage,
    PublishedPackage,
    LocalWasm,
    LocalWasmGz,
    PreviousReceiptArtifact,
    CanonicalWasmStoreDefault,
}

///
/// PreviousArtifactReceiptKindV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PreviousArtifactReceiptKindV1 {
    DeploymentReceipt,
    StagingReceipt,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promotion_readiness_status_owns_text_labels() {
        assert_eq!(PromotionReadinessStatusV1::Ready.label(), "ready");
        assert_eq!(PromotionReadinessStatusV1::Blocked.label(), "blocked");
    }
}
