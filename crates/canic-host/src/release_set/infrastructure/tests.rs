use std::io::Write;

use canic_core::ids::{ReleaseBuildId, ReleaseBuildNonce};
use flate2::{Compression, GzBuilder};

use super::*;

fn release_build(byte: u8) -> ReleaseBuildId {
    ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([byte; 32]))
}

fn wasm(marker: u8) -> Vec<u8> {
    let mut bytes = WASM_MAGIC.to_vec();
    bytes.push(marker);
    bytes
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder.write_all(bytes).expect("write gzip");
    encoder.finish().expect("finish gzip")
}

fn owned_inputs(release_build_id: ReleaseBuildId) -> Vec<OwnedInput> {
    vec![
        OwnedInput::new(
            CanicInfrastructureRole::WasmStore,
            "canic-wasm-store",
            release_build_id,
            3,
        ),
        OwnedInput::new(
            CanicInfrastructureRole::FleetCoordinator,
            "canic-control-plane",
            release_build_id,
            1,
        ),
        OwnedInput::new(
            CanicInfrastructureRole::FleetSubnetRoot,
            "canic-control-plane",
            release_build_id,
            2,
        ),
    ]
}

struct OwnedInput {
    role: CanicInfrastructureRole,
    package: String,
    release_build_id: ReleaseBuildId,
    wasm_relative_path: String,
    wasm: Vec<u8>,
    wasm_gz_relative_path: String,
    wasm_gz: Vec<u8>,
}

impl OwnedInput {
    fn new(
        role: CanicInfrastructureRole,
        package: &str,
        release_build_id: ReleaseBuildId,
        marker: u8,
    ) -> Self {
        let wasm = wasm(marker);
        Self {
            role,
            package: package.to_string(),
            release_build_id,
            wasm_relative_path: format!(".icp/local/canisters/{0}/{0}.wasm", role.as_str()),
            wasm_gz_relative_path: format!(".icp/local/canisters/{0}/{0}.wasm.gz", role.as_str()),
            wasm_gz: gzip(&wasm),
            wasm,
        }
    }

    fn borrowed(&self) -> CanicInfrastructureArtifactInput<'_> {
        CanicInfrastructureArtifactInput {
            role: self.role,
            package: &self.package,
            release_build_id: self.release_build_id,
            wasm_relative_path: &self.wasm_relative_path,
            wasm: &self.wasm,
            wasm_gz_relative_path: &self.wasm_gz_relative_path,
            wasm_gz: &self.wasm_gz,
            candid_sha256: [3; 32],
            protocol_profile_digest: canic_core::role_contract::ProtocolProfileDigest::from_bytes(
                [4; 32],
            ),
        }
    }
}

fn compile(
    release_build_id: ReleaseBuildId,
    inputs: &[OwnedInput],
) -> Result<CanicInfrastructureArtifactManifest, CanicInfrastructureArtifactManifestError> {
    CanicInfrastructureArtifactManifest::compile(
        release_build_id,
        &inputs.iter().map(OwnedInput::borrowed).collect::<Vec<_>>(),
    )
}

#[test]
fn compiler_derives_one_canonical_entry_per_infrastructure_role() {
    let release_build_id = release_build(1);
    let inputs = owned_inputs(release_build_id);
    let manifest = compile(release_build_id, &inputs).expect("compile manifest");

    assert_eq!(
        manifest
            .entries
            .iter()
            .map(|entry| entry.role)
            .collect::<Vec<_>>(),
        REQUIRED_INFRASTRUCTURE_ROLES,
    );
    for entry in &manifest.entries {
        let input = inputs
            .iter()
            .find(|input| input.role == entry.role)
            .expect("role input");
        assert_eq!(entry.release_build_id, release_build_id);
        assert_eq!(entry.package, input.package);
        assert_eq!(entry.wasm_size_bytes, input.wasm.len() as u64);
        assert_eq!(entry.wasm_sha256_hex, sha256_hex(&input.wasm));
        assert_eq!(entry.wasm_gz_size_bytes, input.wasm_gz.len() as u64);
        assert_eq!(entry.wasm_gz_sha256_hex, sha256_hex(&input.wasm_gz));
    }

    let canonical = manifest.canonical_bytes().expect("canonical bytes");
    assert_eq!(
        serde_json::from_slice::<CanicInfrastructureArtifactManifest>(&canonical)
            .expect("decode canonical manifest"),
        manifest,
    );
    let expected_digest: [u8; 32] = Sha256::digest(canonical).into();
    assert_eq!(manifest.digest().expect("manifest digest"), expected_digest);
    assert_eq!(
        canic_core::cdk::utils::hash::hex_bytes(expected_digest),
        "dfb01e65754306204b60a99fbf22e4dd56eb8c95e3f14655e2825049e9647886",
    );
}

#[test]
fn manifest_rejects_missing_duplicate_and_reordered_roles() {
    let release_build_id = release_build(2);
    let mut inputs = owned_inputs(release_build_id);
    inputs.pop();
    assert!(matches!(
        compile(release_build_id, &inputs),
        Err(CanicInfrastructureArtifactManifestError::InfrastructureRoleSet { .. })
    ));

    let mut inputs = owned_inputs(release_build_id);
    inputs[0].role = CanicInfrastructureRole::FleetSubnetRoot;
    assert!(matches!(
        compile(release_build_id, &inputs),
        Err(CanicInfrastructureArtifactManifestError::InfrastructureRoleSet { .. })
    ));

    let inputs = owned_inputs(release_build_id);
    let mut manifest = compile(release_build_id, &inputs).expect("compile manifest");
    manifest.entries.swap(0, 1);
    assert!(matches!(
        manifest.validate(),
        Err(CanicInfrastructureArtifactManifestError::InfrastructureRoleSet { .. })
    ));
}

#[test]
fn compiler_rejects_cross_build_and_representation_mismatch() {
    let release_build_id = release_build(3);
    let mut inputs = owned_inputs(release_build_id);
    inputs[0].release_build_id = release_build(4);
    assert!(matches!(
        compile(release_build_id, &inputs),
        Err(CanicInfrastructureArtifactManifestError::ReleaseBuildMismatch { .. })
    ));

    let mut inputs = owned_inputs(release_build_id);
    inputs[0].wasm_gz = gzip(&wasm(99));
    assert!(matches!(
        compile(release_build_id, &inputs),
        Err(CanicInfrastructureArtifactManifestError::RepresentationMismatch { .. })
    ));
}

#[test]
fn compiler_rejects_invalid_wasm_gzip_package_and_paths() {
    let release_build_id = release_build(5);

    let mut inputs = owned_inputs(release_build_id);
    inputs[0].wasm = b"not-wasm".to_vec();
    assert!(matches!(
        compile(release_build_id, &inputs),
        Err(CanicInfrastructureArtifactManifestError::InvalidWasm { .. })
    ));

    let mut inputs = owned_inputs(release_build_id);
    inputs[0].wasm_gz = b"not-gzip".to_vec();
    assert!(matches!(
        compile(release_build_id, &inputs),
        Err(CanicInfrastructureArtifactManifestError::InvalidGzip { .. })
    ));

    let mut inputs = owned_inputs(release_build_id);
    inputs[0].package = "../package".to_string();
    assert!(matches!(
        compile(release_build_id, &inputs),
        Err(CanicInfrastructureArtifactManifestError::InvalidPackage { .. })
    ));

    let mut inputs = owned_inputs(release_build_id);
    inputs[0].wasm_relative_path = "../artifact.wasm".to_string();
    assert!(matches!(
        compile(release_build_id, &inputs),
        Err(CanicInfrastructureArtifactManifestError::InvalidPath { .. })
    ));
}

#[test]
fn validation_rejects_noncanonical_hashes_and_duplicate_paths() {
    let release_build_id = release_build(6);
    let inputs = owned_inputs(release_build_id);
    let mut manifest = compile(release_build_id, &inputs).expect("compile manifest");
    manifest.entries[0].wasm_sha256_hex = "AA".repeat(32);
    assert!(matches!(
        manifest.validate(),
        Err(CanicInfrastructureArtifactManifestError::InvalidSha256 { .. })
    ));

    let mut manifest = compile(release_build_id, &inputs).expect("compile manifest");
    manifest.entries[1].wasm_relative_path = manifest.entries[0].wasm_relative_path.clone();
    assert!(matches!(
        manifest.validate(),
        Err(CanicInfrastructureArtifactManifestError::DuplicateArtifactPath { .. })
    ));
}
