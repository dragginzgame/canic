use super::operations::InstallPhaseOperation;
use crate::deployment_truth::{
    ArtifactTransportV1, ObservationStatusV1, StagingReceiptV1, VerifiedPostconditionV1,
    staging_receipt_evidence,
};
use crate::release_set::{RootReleaseSetManifest, stage_root_release_set};
use std::path::Path;

pub(super) struct StageReleaseSetOperation<'a> {
    icp_root: &'a Path,
    network: &'a str,
    root_canister_id: &'a str,
    manifest_path: &'a Path,
    manifest: RootReleaseSetManifest,
}

impl<'a> StageReleaseSetOperation<'a> {
    pub(super) const fn new(
        icp_root: &'a Path,
        network: &'a str,
        root_canister_id: &'a str,
        manifest_path: &'a Path,
        manifest: RootReleaseSetManifest,
    ) -> Self {
        Self {
            icp_root,
            network,
            root_canister_id,
            manifest_path,
            manifest,
        }
    }
}

impl InstallPhaseOperation for StageReleaseSetOperation<'_> {
    fn phase(&self) -> &'static str {
        "stage_release_set"
    }

    fn attempted_action(&self) -> &'static str {
        "stage root release set"
    }

    fn evidence(&self) -> Vec<String> {
        current_install_staging_evidence(self.root_canister_id, self.manifest_path, &self.manifest)
    }

    fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        stage_root_release_set(
            self.icp_root,
            self.network,
            self.root_canister_id,
            &self.manifest,
        )
    }
}

pub(super) fn current_install_staging_evidence(
    root_canister_id: &str,
    manifest_path: &Path,
    manifest: &RootReleaseSetManifest,
) -> Vec<String> {
    let mut evidence = vec![
        format!("root_canister:{root_canister_id}"),
        format!("manifest_path:{}", manifest_path.display()),
        format!("release_version:{}", manifest.release_version),
    ];
    let staging_receipts = current_install_staging_receipts(root_canister_id, manifest);
    evidence.extend(staging_receipt_evidence(&staging_receipts));
    evidence
}

fn current_install_staging_receipts(
    root_canister_id: &str,
    manifest: &RootReleaseSetManifest,
) -> Vec<StagingReceiptV1> {
    manifest
        .entries
        .iter()
        .map(|entry| StagingReceiptV1 {
            schema_version: crate::deployment_truth::DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            role: entry.role.clone(),
            artifact_identity: format!(
                "{}:{}:{}",
                entry.template_id, manifest.release_version, entry.payload_sha256_hex
            ),
            transport: ArtifactTransportV1::WasmStore,
            wasm_store_locator: Some(format!("root:{root_canister_id}:bootstrap")),
            prepared_chunk_hashes: entry.chunk_sha256_hex.clone(),
            published_chunk_count: entry.chunk_sha256_hex.len(),
            verified_postcondition: VerifiedPostconditionV1 {
                status: ObservationStatusV1::Observed,
                evidence: vec![
                    format!("payload_sha256:{}", entry.payload_sha256_hex),
                    format!("payload_size_bytes:{}", entry.payload_size_bytes),
                    format!("chunk_size_bytes:{}", entry.chunk_size_bytes),
                    format!("chunk_count:{}", entry.chunk_sha256_hex.len()),
                ],
            },
        })
        .collect()
}
