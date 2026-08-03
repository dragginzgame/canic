//! Module: install_root::fleet_subnet_root_store_bootstrap
//!
//! Responsibility: stage, bootstrap, and independently verify every planned root-local Store.
//! Does not own: root creation, Registry registration, or runtime activation.
//! Boundary: root release-set and artifact staging is exact-idempotent; each journal crosses the
//! Store effect only after all roots have independently reached installed `Prepared` authority.

use super::fleet_subnet_root_install_journal::{
    FleetSubnetRootInstallPhase, PlanFleetSubnetRootInstallRequest, ResolvedFleetSubnetRootInstall,
    begin_store_adoption, begin_store_bootstrap, begin_store_staging,
    expected_wasm_store_authority, plan_fleet_subnet_root_install, record_store_adopted,
    record_store_bootstrapped, record_store_staged, record_store_verified,
};
use super::operations::{call_with_arg, query_with_arg};
use crate::{
    durable_io::{RegularFileReadError, read_optional_regular_bytes},
    fleet_install_plan::{PersistedFleetInstallPlan, PersistedFleetSubnetRootReleaseSet},
    icp::{IcpCli, LocalReplicaTarget},
    release_set::{
        AppConfigSnapshot, ApplicationArtifactEntry,
        load_persisted_canic_infrastructure_artifact_manifest, resolve_release_artifact_path,
    },
};
use candid::Principal;
use canic_control_plane::{
    dto::template::{
        TemplateChunkInput, TemplateChunkSetInfoResponse, TemplateChunkSetPrepareInput,
        TemplateManifestInput,
    },
    ids::{
        TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion, WasmStoreBinding,
    },
};
use canic_core::{
    cdk::utils::hash::{hex_bytes, wasm_hash},
    dto::fleet_subnet_root::{
        FleetSubnetWasmStoreAdoptionRequest, FleetSubnetWasmStoreAdoptionResponse,
    },
    dto::root_store::{
        ROOT_STORE_ARTIFACT_TEMPLATE_PREFIX, ROOT_STORE_RELEASE_SET_MANIFEST_MAX_BYTES,
        ROOT_STORE_RELEASE_SET_TEMPLATE_PREFIX, RootStoreBootstrapRequest,
        RootStoreBootstrapResponse,
    },
    protocol,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const MAX_STORE_TRANSITIONS: usize = 10;

#[derive(Debug, ThisError)]
enum RootStoreBootstrapError {
    #[error("root Store workflow reached pre-verification phase {0:?}")]
    RootNotVerified(FleetSubnetRootInstallPhase),

    #[error("root release-set manifest has no matching plan for Subnet {0}")]
    MissingReleaseSet(canic_core::ids::SubnetId),

    #[error("root release-set runtime projection differs from canonical host bytes")]
    RuntimeProjectionMismatch,

    #[error("root release-set manifest exceeds the runtime byte bound")]
    ManifestTooLarge,

    #[error("root release-set manifest digest differs from immutable plan authority")]
    ManifestDigestMismatch,

    #[error("application artifact is missing: {0}")]
    ArtifactMissing(PathBuf),

    #[error("application artifact is not a regular no-follow file: {0}")]
    ArtifactUnsafe(PathBuf),

    #[error("application artifact {path} has size {actual}, expected {expected}")]
    ArtifactSize {
        path: PathBuf,
        expected: u64,
        actual: usize,
    },

    #[error("application artifact {path} has SHA-256 {actual}, expected {expected}")]
    ArtifactHash {
        path: PathBuf,
        expected: String,
        actual: String,
    },

    #[error("root Store staging prepared different chunk hashes")]
    PreparedChunkSetMismatch,

    #[error("live root Store evidence differs from the journalled bootstrap result")]
    LiveEvidenceMismatch,

    #[error("root Store adoption update and separately queried receipt differ")]
    AdoptionResponseMismatch,

    #[error("root Store workflow exceeded its bounded phase transitions")]
    TransitionBoundExceeded,
}

pub(super) fn bootstrap_and_verify_fleet_subnet_root_stores(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    config_path: &Path,
    fleet_install_plan: &PersistedFleetInstallPlan,
    coordinator: Principal,
    install_operation_id: [u8; 32],
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(config_path)?;
    let component_topology = config.model().compile_component_topology()?;
    let infrastructure_manifest = load_persisted_canic_infrastructure_artifact_manifest(
        icp_root,
        fleet_install_plan.plan.release_build_id,
    )?;

    for root_plan in &fleet_install_plan.plan.fleet_subnet_roots {
        let release_set = fleet_install_plan
            .root_release_sets
            .iter()
            .find(|release_set| release_set.placement_subnet == root_plan.placement_subnet)
            .ok_or(RootStoreBootstrapError::MissingReleaseSet(
                root_plan.placement_subnet,
            ))?;
        let current = plan_fleet_subnet_root_install(PlanFleetSubnetRootInstallRequest {
            fleet_install_plan,
            infrastructure_manifest: &infrastructure_manifest,
            coordinator,
            install_operation_id,
            component_topology: component_topology.clone(),
            root_plan,
        })?;
        drive_store_bootstrap(icp_root, environment, local_replica, release_set, current)?;
    }

    Ok(())
}

fn drive_store_bootstrap(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    release_set: &PersistedFleetSubnetRootReleaseSet,
    mut current: ResolvedFleetSubnetRootInstall,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_bytes = canonical_manifest_bytes(release_set)?;
    let request = RootStoreBootstrapRequest {
        manifest_payload_size_bytes: manifest_bytes.len() as u64,
    };

    for _ in 0..MAX_STORE_TRANSITIONS {
        current = match current.journal.phase {
            FleetSubnetRootInstallPhase::InfrastructureVerified => begin_store_adoption(&current)?,
            FleetSubnetRootInstallPhase::StoreAdoptionInFlight => {
                let evidence = adopt_store(icp_root, environment, local_replica, &current)?;
                record_store_adopted(&current, evidence)?
            }
            FleetSubnetRootInstallPhase::StoreAdopted => begin_store_staging(&current)?,
            FleetSubnetRootInstallPhase::StoreStaging => {
                stage_release_set(
                    icp_root,
                    environment,
                    local_replica,
                    &current,
                    release_set,
                    &manifest_bytes,
                )?;
                record_store_staged(&current)?
            }
            FleetSubnetRootInstallPhase::StoreStaged => begin_store_bootstrap(&current)?,
            FleetSubnetRootInstallPhase::StoreBootstrapInFlight => {
                let evidence = call_store_bootstrap(
                    icp_root,
                    environment,
                    local_replica,
                    &current,
                    request.clone(),
                )?;
                record_store_bootstrapped(&current, evidence)?
            }
            FleetSubnetRootInstallPhase::StoreBootstrapped => {
                let evidence = query_store_bootstrap_status(
                    icp_root,
                    environment,
                    local_replica,
                    &current,
                    request.clone(),
                )?;
                record_store_verified(&current, evidence)?
            }
            FleetSubnetRootInstallPhase::StoreVerified
            | FleetSubnetRootInstallPhase::RegistryJoinInFlight
            | FleetSubnetRootInstallPhase::RegistryJoined
            | FleetSubnetRootInstallPhase::RegistryJoinVerified
            | FleetSubnetRootInstallPhase::RegistrySyncInFlight
            | FleetSubnetRootInstallPhase::RegistrySynchronized
            | FleetSubnetRootInstallPhase::RegistrySyncVerified
            | FleetSubnetRootInstallPhase::RegistryMirrorActivationInFlight
            | FleetSubnetRootInstallPhase::RegistryMirrorActivated
            | FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified
            | FleetSubnetRootInstallPhase::ComponentRegistryPreparationInFlight
            | FleetSubnetRootInstallPhase::ComponentRegistryPrepared
            | FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified
            | FleetSubnetRootInstallPhase::RootActivationPreparationInFlight
            | FleetSubnetRootInstallPhase::RootActivationPrepared
            | FleetSubnetRootInstallPhase::RootActivationInFlight
            | FleetSubnetRootInstallPhase::RootActivated
            | FleetSubnetRootInstallPhase::RootActivationVerified => {
                let observed = query_store_bootstrap_status(
                    icp_root,
                    environment,
                    local_replica,
                    &current,
                    request,
                )?;
                if current.journal.store_bootstrap.as_ref() != Some(&observed) {
                    return Err(RootStoreBootstrapError::LiveEvidenceMismatch.into());
                }
                return Ok(());
            }
            phase => return Err(RootStoreBootstrapError::RootNotVerified(phase).into()),
        };
    }
    Err(RootStoreBootstrapError::TransitionBoundExceeded.into())
}

fn adopt_store(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<FleetSubnetWasmStoreAdoptionResponse, Box<dyn std::error::Error>> {
    let root = current
        .journal
        .fleet_subnet_root
        .expect("Store adoption follows verified root installation");
    let request = FleetSubnetWasmStoreAdoptionRequest {
        operation_id: current.journal.install_operation_id,
        authority: expected_wasm_store_authority(&current.journal)?,
    };
    let icp = super::install_icp(icp_root, environment, local_replica);
    let updated = call_with_arg::<_, FleetSubnetWasmStoreAdoptionResponse>(
        &icp,
        root,
        protocol::CANIC_FLEET_SUBNET_WASM_STORE_ADOPT,
        &request,
    );
    let observed = query_with_arg::<_, FleetSubnetWasmStoreAdoptionResponse>(
        &icp,
        root,
        protocol::CANIC_FLEET_SUBNET_WASM_STORE_ADOPTION_STATUS,
        &request,
    )?;
    match updated {
        Ok(updated) if updated != observed => {
            return Err(RootStoreBootstrapError::AdoptionResponseMismatch.into());
        }
        Ok(_) | Err(_) => {}
    }
    Ok(observed)
}

pub(super) fn canonical_manifest_bytes(
    release_set: &PersistedFleetSubnetRootReleaseSet,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let host_bytes = serde_json::to_vec(&release_set.manifest)?;
    let runtime_bytes = serde_json::to_vec(&release_set.manifest.root_store_manifest())?;
    if runtime_bytes != host_bytes {
        return Err(RootStoreBootstrapError::RuntimeProjectionMismatch.into());
    }
    if runtime_bytes.is_empty()
        || runtime_bytes.len() as u64 > ROOT_STORE_RELEASE_SET_MANIFEST_MAX_BYTES
    {
        return Err(RootStoreBootstrapError::ManifestTooLarge.into());
    }
    let digest: [u8; 32] = Sha256::digest(&runtime_bytes).into();
    if &digest != release_set.digest.as_bytes() {
        return Err(RootStoreBootstrapError::ManifestDigestMismatch.into());
    }
    Ok(runtime_bytes)
}

fn stage_release_set(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    current: &ResolvedFleetSubnetRootInstall,
    release_set: &PersistedFleetSubnetRootReleaseSet,
    manifest_bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let root = current
        .journal
        .fleet_subnet_root
        .expect("Store staging follows verified root installation");
    let icp = super::install_icp(icp_root, environment, local_replica);
    let version = TemplateVersion::owned(release_set.manifest.release_build_id.to_string());
    let manifest_template_id = TemplateId::owned(format!(
        "{ROOT_STORE_RELEASE_SET_TEMPLATE_PREFIX}{}",
        release_set.digest
    ));
    stage_chunk_set(
        &icp,
        root,
        manifest_template_id,
        version.clone(),
        release_set.digest.as_bytes().to_vec(),
        manifest_bytes,
    )?;

    let artifacts = unique_artifacts(release_set)?;
    for (role, artifact) in artifacts {
        let bytes = load_artifact_bytes(icp_root, artifact)?;
        let template_id = TemplateId::owned(format!("{ROOT_STORE_ARTIFACT_TEMPLATE_PREFIX}{role}"));
        let payload_hash = wasm_hash(&bytes);
        call_with_arg::<_, ()>(
            &icp,
            root,
            protocol::CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
            &TemplateManifestInput {
                template_id: template_id.clone(),
                role,
                version: version.clone(),
                payload_hash: payload_hash.clone(),
                payload_size_bytes: bytes.len() as u64,
                store_binding: WasmStoreBinding::new("bootstrap"),
                chunking_mode: TemplateChunkingMode::Chunked,
                manifest_state: TemplateManifestState::Approved,
                approved_at: Some(0),
                created_at: 0,
            },
        )?;
        stage_chunk_set(
            &icp,
            root,
            template_id,
            version.clone(),
            payload_hash,
            &bytes,
        )?;
    }
    Ok(())
}

fn unique_artifacts(
    release_set: &PersistedFleetSubnetRootReleaseSet,
) -> Result<
    BTreeMap<canic_core::ids::CanisterRole, &ApplicationArtifactEntry>,
    Box<dyn std::error::Error>,
> {
    let mut artifacts = BTreeMap::new();
    for entry in &release_set.manifest.entries {
        match artifacts.insert(entry.artifact.role.clone(), &entry.artifact) {
            Some(existing) if existing != &entry.artifact => {
                return Err("one role has conflicting root release-set artifacts".into());
            }
            _ => {}
        }
    }
    Ok(artifacts)
}

fn load_artifact_bytes(
    icp_root: &Path,
    artifact: &ApplicationArtifactEntry,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let path = resolve_release_artifact_path(icp_root, &artifact.wasm_gz_relative_path)?;
    let bytes = match read_optional_regular_bytes(&path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return Err(RootStoreBootstrapError::ArtifactMissing(path).into()),
        Err(RegularFileReadError::NotRegular) => {
            return Err(RootStoreBootstrapError::ArtifactUnsafe(path).into());
        }
        Err(RegularFileReadError::Io(source)) => return Err(source.into()),
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "application artifact reads are unsupported",
            )
            .into());
        }
    };
    if bytes.len() as u64 != artifact.wasm_gz_size_bytes {
        return Err(RootStoreBootstrapError::ArtifactSize {
            path,
            expected: artifact.wasm_gz_size_bytes,
            actual: bytes.len(),
        }
        .into());
    }
    let actual = hex_bytes(Sha256::digest(&bytes));
    if actual != artifact.wasm_gz_sha256_hex {
        return Err(RootStoreBootstrapError::ArtifactHash {
            path,
            expected: artifact.wasm_gz_sha256_hex.clone(),
            actual,
        }
        .into());
    }
    Ok(bytes)
}

fn stage_chunk_set(
    icp: &IcpCli,
    root: Principal,
    template_id: TemplateId,
    version: TemplateVersion,
    payload_hash: Vec<u8>,
    bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let chunks = bytes
        .chunks(canic_core::CANIC_WASM_CHUNK_BYTES)
        .map(<[u8]>::to_vec)
        .collect::<Vec<_>>();
    let chunk_hashes = chunks
        .iter()
        .map(|chunk| wasm_hash(chunk))
        .collect::<Vec<_>>();
    let prepared = call_with_arg::<_, TemplateChunkSetInfoResponse>(
        icp,
        root,
        protocol::CANIC_TEMPLATE_PREPARE_ADMIN,
        &TemplateChunkSetPrepareInput {
            template_id: template_id.clone(),
            version: version.clone(),
            payload_hash,
            payload_size_bytes: bytes.len() as u64,
            chunk_hashes: chunk_hashes.clone(),
        },
    )?;
    if prepared.chunk_hashes != chunk_hashes {
        return Err(RootStoreBootstrapError::PreparedChunkSetMismatch.into());
    }
    for (chunk_index, bytes) in chunks.into_iter().enumerate() {
        call_with_arg::<_, ()>(
            icp,
            root,
            protocol::CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
            &TemplateChunkInput {
                template_id: template_id.clone(),
                version: version.clone(),
                chunk_index: u32::try_from(chunk_index)?,
                bytes,
            },
        )?;
    }
    Ok(())
}

fn call_store_bootstrap(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    current: &ResolvedFleetSubnetRootInstall,
    request: RootStoreBootstrapRequest,
) -> Result<RootStoreBootstrapResponse, Box<dyn std::error::Error>> {
    let root = current
        .journal
        .fleet_subnet_root
        .expect("Store bootstrap follows verified root installation");
    call_with_arg(
        &super::install_icp(icp_root, environment, local_replica),
        root,
        protocol::CANIC_ROOT_STORE_BOOTSTRAP,
        &request,
    )
    .map_err(Into::into)
}

fn query_store_bootstrap_status(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    current: &ResolvedFleetSubnetRootInstall,
    request: RootStoreBootstrapRequest,
) -> Result<RootStoreBootstrapResponse, Box<dyn std::error::Error>> {
    let root = current
        .journal
        .fleet_subnet_root
        .expect("Store verification follows verified root installation");
    query_with_arg(
        &super::install_icp(icp_root, environment, local_replica),
        root,
        protocol::CANIC_ROOT_STORE_BOOTSTRAP_STATUS,
        &request,
    )
    .map_err(Into::into)
}
