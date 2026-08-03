//! Module: ops::storage::fleet_activation
//!
//! Responsibility: validate, initialize, and project the protected Fleet activation record.
//! Does not own: lifecycle orchestration, embedded build lookup, endpoint policy, or timers.
//! Boundary: initialization writes `Prepared` once; status rejects invalid role/state projections.

mod mapper;

#[cfg(test)]
use crate::storage::stable::fleet_activation::FleetActivationData;
use crate::{
    config::ComponentTopology,
    dto::fleet_subnet_root::{FleetSubnetRootAuthority, FleetSubnetRootInitArgs},
    dto::{
        component_registry::{
            ComponentDirectoryHead, ComponentDirectoryProvenance,
            ComponentRuntimeActivationEvidence, ComponentRuntimeActivationRequest,
            ComponentRuntimeDirectoryAuthority, ComponentRuntimeDirectoryPreparationRequest,
            ComponentRuntimeDirectorySynchronizationRequest, ComponentRuntimePhase,
            ComponentRuntimeStatusResponse,
        },
        fleet_activation::{
            FleetActivationIdentity, FleetActivationRequest, FleetActivationStatusResponse,
            FleetCascadeActivationEvidence, FleetCascadeManifestEntry,
            FleetCredentialGenerationRef, FleetCredentialGenerationRequest,
            FleetCredentialManifest,
        },
        fleet_registry::{
            FleetDirectoryProvenance, FleetDirectorySnapshot, FleetRegistryVersion,
            FleetSubnetRootDirectoryEntry, FleetSubnetRootStatus,
        },
    },
    ids::{AppId, FleetBinding, ManagedCanisterBinding, ReleaseBuildId},
    model::fleet_activation::{
        NonrootInstallIdentity, PrepareFleetActivationError, PreparedFleetActivation,
        PreparedFleetSubnetRootAuthority, RootInstallIdentity, prepare_nonroot_install,
        prepare_root_install,
    },
    storage::stable::fleet_activation::{
        ComponentDirectoryHeadRecord, ComponentDirectoryProvenanceRecord,
        ComponentRuntimeActivationRecord, ComponentRuntimeDirectoryAuthorityRecord,
        ComponentRuntimeDirectoryRecord, ComponentRuntimeRecord, FleetActivation,
        FleetActivationEvidenceRecord, FleetActivationIdentityRecord, FleetActivationRecord,
        FleetActivationStateRecord, FleetCascadeActivationEvidenceRecord,
        FleetCascadeManifestEntryRecord, FleetCredentialGenerationRefRecord,
        FleetCredentialManifestEntryRecord, FleetCredentialManifestRecord,
        FleetDirectoryProvenanceRecord, FleetDirectorySnapshotRecord, FleetRegistryVersionRecord,
        FleetSubnetRootAuthorityRecord, FleetSubnetRootDirectoryEntryRecord,
        FleetSubnetRootStatusRecord, MAX_FLEET_ACTIVATION_RECORD_BYTES,
    },
    view::fleet_activation::{ComponentRuntimeActivationTransition, FleetActivationTransition},
};
use thiserror::Error as ThisError;

///
/// FleetActivationOpsError
///

#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum FleetActivationOpsError {
    #[error(transparent)]
    Admission(#[from] PrepareFleetActivationError),

    #[error("failed to encode protected Fleet activation record: {0}")]
    Encode(String),

    #[error("protected Fleet activation record exceeds {maximum} bytes: observed {observed} bytes")]
    RecordTooLarge { maximum: usize, observed: usize },

    #[error("protected Fleet activation record is already initialized")]
    AlreadyInitialized,

    #[error("protected Fleet activation record is not initialized")]
    NotInitialized,

    #[error("protected Fleet activation record is invalid: {reason}")]
    InvalidRecord { reason: String },

    #[error("protected Fleet activation is not Active")]
    NotActive,

    #[error("Fleet activation identity does not match the protected operation")]
    IdentityMismatch,

    #[error("Fleet activation evidence does not match protected state")]
    EvidenceMismatch,

    #[error("Fleet activation transition is invalid: {reason}")]
    InvalidTransition { reason: String },
}

///
/// FleetActivationOps
///

pub struct FleetActivationOps;

/// Fully validated activation-evidence replacement ready for an infallible commit.
pub struct PreparedFleetActivationSnapshot(Option<FleetActivationRecord>);

impl FleetActivationOps {
    pub(crate) fn initialize_root_prepared(
        input: FleetSubnetRootInitArgs,
        embedded_release_build_id: ReleaseBuildId,
        configured_app: &AppId,
        component_topology: &ComponentTopology,
        root_canister: candid::Principal,
    ) -> Result<FleetActivationIdentity, FleetActivationOpsError> {
        let prepared = prepare_root_install(
            RootInstallIdentity {
                binding: input.authority.binding,
                initial_release_set: input.authority.initial_release_set,
                install_id: input.install_id,
                expected_module_hash: input.authority.expected_module_hash,
            },
            embedded_release_build_id,
            configured_app,
            component_topology,
            root_canister,
        )?;
        initialize_prepared(prepared, None, None)
    }

    pub(crate) fn initialize_nonroot_prepared(
        fleet: FleetBinding,
        install_id: [u8; 32],
        release_build_id: ReleaseBuildId,
        embedded_release_build_id: ReleaseBuildId,
        component_binding: Option<ManagedCanisterBinding>,
        application_init_args: Option<Vec<u8>>,
    ) -> Result<FleetActivationIdentity, FleetActivationOpsError> {
        let prepared = prepare_nonroot_install(
            NonrootInstallIdentity {
                fleet,
                install_id,
                release_build_id,
            },
            embedded_release_build_id,
        )?;
        initialize_prepared(prepared, component_binding, application_init_args)
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn snapshot() -> FleetActivationData {
        FleetActivation::export()
    }

    pub(crate) fn status(
        is_root: bool,
    ) -> Result<FleetActivationStatusResponse, FleetActivationOpsError> {
        let record = FleetActivation::get().ok_or(FleetActivationOpsError::NotInitialized)?;
        mapper::record_to_status(record, is_root)
    }

    pub(crate) fn fleet_binding() -> Result<FleetBinding, FleetActivationOpsError> {
        let record = FleetActivation::get().ok_or(FleetActivationOpsError::NotInitialized)?;
        let identity = match record.state {
            FleetActivationStateRecord::Prepared { identity, .. }
            | FleetActivationStateRecord::Active { identity, .. } => identity,
        };
        Ok(identity.fleet)
    }

    pub(crate) fn component_runtime_status()
    -> Result<ComponentRuntimeStatusResponse, FleetActivationOpsError> {
        let record = FleetActivation::get().ok_or(FleetActivationOpsError::NotInitialized)?;
        component_runtime_status(record)
    }

    pub(crate) fn prepare_component_runtime_directory(
        request: ComponentRuntimeDirectoryPreparationRequest,
        authority_hash: [u8; 32],
        direct_children_hash: [u8; 32],
    ) -> Result<ComponentRuntimeStatusResponse, FleetActivationOpsError> {
        let mut record = FleetActivation::get().ok_or(FleetActivationOpsError::NotInitialized)?;
        let (operation_id, is_prepared) = match &record.state {
            FleetActivationStateRecord::Prepared { identity, .. } => (identity.operation_id, true),
            FleetActivationStateRecord::Active { identity, .. } => (identity.operation_id, false),
        };
        if operation_id != request.operation_id {
            return Err(FleetActivationOpsError::IdentityMismatch);
        }
        let component_runtime = record.component_runtime.as_mut().ok_or_else(|| {
            FleetActivationOpsError::InvalidRecord {
                reason: "protected non-root is not a managed Component-tree runtime".to_string(),
            }
        })?;
        let next = ComponentRuntimeDirectoryRecord {
            authority: component_runtime_directory_dto_to_record(request.authority),
            authority_hash,
            direct_children_hash,
        };
        if component_runtime
            .activation
            .as_ref()
            .is_some_and(|activation| activation.directory == next)
        {
            return component_runtime_preparation_status(record);
        }
        match &component_runtime.directory {
            Some(existing) if existing == &next => {
                return component_runtime_status(record);
            }
            Some(_) => return Err(FleetActivationOpsError::EvidenceMismatch),
            None if !is_prepared => {
                return Err(FleetActivationOpsError::InvalidTransition {
                    reason:
                        "Component Directory authority must be prepared before runtime activation"
                            .to_string(),
                });
            }
            None => component_runtime.directory = Some(next),
        }
        replace_record(record.clone())?;
        component_runtime_status(record)
    }

    pub(crate) fn synchronize_component_runtime_directory(
        request: ComponentRuntimeDirectorySynchronizationRequest,
        authority_hash: [u8; 32],
        direct_children_hash: [u8; 32],
    ) -> Result<ComponentRuntimeStatusResponse, FleetActivationOpsError> {
        let mut record = FleetActivation::get().ok_or(FleetActivationOpsError::NotInitialized)?;
        let operation_id = match &record.state {
            FleetActivationStateRecord::Prepared { .. } => {
                return Err(FleetActivationOpsError::InvalidTransition {
                    reason:
                        "current Component Directory synchronization requires an Active runtime"
                            .to_string(),
                });
            }
            FleetActivationStateRecord::Active { identity, .. } => identity.operation_id,
        };
        if operation_id != request.operation_id {
            return Err(FleetActivationOpsError::IdentityMismatch);
        }
        let component_runtime = record.component_runtime.as_mut().ok_or_else(|| {
            FleetActivationOpsError::InvalidRecord {
                reason: "protected non-root is not a managed Component-tree runtime".to_string(),
            }
        })?;
        if component_runtime.activation.is_none() {
            return Err(FleetActivationOpsError::InvalidRecord {
                reason: "Active Component runtime has no immutable activation receipt".to_string(),
            });
        }
        let current = component_runtime.directory.as_ref().ok_or_else(|| {
            FleetActivationOpsError::InvalidRecord {
                reason: "Active Component runtime has no current Directory authority".to_string(),
            }
        })?;
        let next = ComponentRuntimeDirectoryRecord {
            authority: component_runtime_directory_dto_to_record(request.authority),
            authority_hash,
            direct_children_hash,
        };
        if current == &next {
            return component_runtime_status(record);
        }
        let current_provenance = &current.authority.component.provenance;
        let next_provenance = &next.authority.component.provenance;
        let current_fleet_revision = current.authority.fleet.provenance.registry.revision;
        let next_fleet_revision = next.authority.fleet.provenance.registry.revision;
        let component_identity_is_stable = next_provenance.component
            == current_provenance.component
            && next_provenance.source_fleet_subnet_root
                == current_provenance.source_fleet_subnet_root;
        let component_authority_advances = next_provenance.component_registry_revision
            > current_provenance.component_registry_revision
            && next_provenance.component_registry_content_hash
                != current_provenance.component_registry_content_hash
            && next_provenance.synchronized_at_ns > current_provenance.synchronized_at_ns;
        let fleet_authority_is_monotonic = match next_fleet_revision.cmp(&current_fleet_revision) {
            std::cmp::Ordering::Less => false,
            std::cmp::Ordering::Equal => next.authority.fleet == current.authority.fleet,
            std::cmp::Ordering::Greater => true,
        };
        if !component_identity_is_stable
            || !component_authority_advances
            || !fleet_authority_is_monotonic
        {
            return Err(FleetActivationOpsError::EvidenceMismatch);
        }
        component_runtime.directory = Some(next);
        replace_record(record.clone())?;
        component_runtime_status(record)
    }

    pub(crate) fn activate_component_runtime(
        request: ComponentRuntimeActivationRequest,
        activated_at_ns: u64,
    ) -> Result<ComponentRuntimeActivationTransition, FleetActivationOpsError> {
        let mut record = FleetActivation::get().ok_or(FleetActivationOpsError::NotInitialized)?;
        if record.root_authority.is_some() {
            return Err(FleetActivationOpsError::InvalidRecord {
                reason: "Fleet Subnet Root cannot contain Component runtime state".to_string(),
            });
        }
        let (identity, evidence, application_init_args, already_active) = match &record.state {
            FleetActivationStateRecord::Prepared {
                identity,
                evidence,
                application_init_args,
            } => (
                identity.clone(),
                evidence.clone(),
                application_init_args.clone(),
                false,
            ),
            FleetActivationStateRecord::Active {
                identity, evidence, ..
            } => (identity.clone(), evidence.clone(), None, true),
        };
        if identity.operation_id != request.operation_id {
            return Err(FleetActivationOpsError::IdentityMismatch);
        }
        let component_runtime = record.component_runtime.as_ref().ok_or_else(|| {
            FleetActivationOpsError::InvalidRecord {
                reason: "protected non-root is not a managed Component-tree runtime".to_string(),
            }
        })?;
        let directory = component_runtime.directory.as_ref().ok_or_else(|| {
            FleetActivationOpsError::InvalidTransition {
                reason: "Component Directory authority must be prepared before runtime activation"
                    .to_string(),
            }
        })?;
        if request.directory_authority_hash == [0; 32] {
            return Err(FleetActivationOpsError::EvidenceMismatch);
        }
        let obsolete_fleet_evidence = evidence.cascade.is_some() || evidence.credential.is_some();
        let obsolete_runtime_evidence = record.prepared_state_snapshot_hash.is_some()
            || record.prepared_topology_snapshot_hash.is_some()
            || record.cascade_manifest.is_some()
            || !record.credential_manifests.is_empty();
        if obsolete_fleet_evidence || obsolete_runtime_evidence {
            return Err(FleetActivationOpsError::InvalidRecord {
                reason: "managed Component runtime retains obsolete cascade or credential evidence"
                    .to_string(),
            });
        }

        if already_active {
            return replay_component_runtime_activation(record, request);
        }
        if directory.authority_hash != request.directory_authority_hash {
            return Err(FleetActivationOpsError::EvidenceMismatch);
        }
        let activation_directory = directory.clone();
        if activated_at_ns == 0 {
            return Err(FleetActivationOpsError::InvalidTransition {
                reason: "Component runtime activation timestamp must be positive".to_string(),
            });
        }
        if component_runtime.activation.is_some() {
            return Err(FleetActivationOpsError::InvalidRecord {
                reason: "Prepared Component runtime already retains activation evidence"
                    .to_string(),
            });
        }

        record.state = FleetActivationStateRecord::Active {
            identity,
            evidence,
            activated_at_ns,
        };
        let component_runtime = record.component_runtime.as_mut().ok_or_else(|| {
            FleetActivationOpsError::InvalidRecord {
                reason: "validated Component runtime record disappeared".to_string(),
            }
        })?;
        component_runtime.activation = Some(ComponentRuntimeActivationRecord {
            directory: activation_directory,
            activated_at_ns,
        });
        replace_record(record.clone())?;
        Ok(ComponentRuntimeActivationTransition {
            status: component_runtime_status(record)?,
            transitioned: true,
            application_init_args,
        })
    }

    pub(crate) fn root_authority() -> Result<FleetSubnetRootAuthority, FleetActivationOpsError> {
        let record = FleetActivation::get().ok_or(FleetActivationOpsError::NotInitialized)?;
        record
            .root_authority
            .map(root_authority_record_to_dto)
            .ok_or_else(|| FleetActivationOpsError::InvalidRecord {
                reason: "protected Fleet activation record has no Fleet Subnet Root authority"
                    .to_string(),
            })
    }

    pub(crate) fn require_active(is_root: bool) -> Result<(), FleetActivationOpsError> {
        let status = Self::status(is_root)?;
        if status.phase != crate::dto::fleet_activation::FleetActivationPhase::Active {
            return Err(FleetActivationOpsError::NotActive);
        }
        Ok(())
    }

    pub(crate) fn prepare_root(
        cascade_manifest: Vec<FleetCascadeManifestEntry>,
        cascade_manifest_hash: [u8; 32],
        credential: FleetCredentialGenerationRef,
        credential_manifest: FleetCredentialManifest,
    ) -> Result<FleetActivationStatusResponse, FleetActivationOpsError> {
        let mut record = FleetActivation::get().ok_or(FleetActivationOpsError::NotInitialized)?;
        let FleetActivationStateRecord::Prepared {
            identity, evidence, ..
        } = &mut record.state
        else {
            return Self::status(true);
        };
        let credential_is_versioned =
            credential.generation > 0 && credential_manifest.generation == credential.generation;
        let credential_scope_matches = credential_manifest.fleet == identity.fleet.fleet
            && credential_manifest.activation_id == identity.operation_id;
        if !credential_is_versioned || !credential_scope_matches {
            return Err(FleetActivationOpsError::IdentityMismatch);
        }

        let source = FleetCascadeActivationEvidenceRecord::Source {
            cascade_manifest_hash,
        };
        let credential_record = FleetCredentialGenerationRefRecord {
            generation: credential.generation,
            manifest_hash: credential.manifest_hash,
        };
        let manifest_records = cascade_manifest
            .into_iter()
            .map(|entry| FleetCascadeManifestEntryRecord {
                principal: entry.principal,
                state_snapshot_hash: entry.state_snapshot_hash,
                topology_snapshot_hash: entry.topology_snapshot_hash,
            })
            .collect::<Vec<_>>();
        let credential_manifest_record = credential_manifest_to_record(credential_manifest);

        if let Some(existing) = &evidence.cascade
            && existing != &source
        {
            return Err(FleetActivationOpsError::EvidenceMismatch);
        }
        if let Some(existing) = &evidence.credential
            && existing != &credential_record
        {
            return Err(FleetActivationOpsError::EvidenceMismatch);
        }
        if let Some(existing) = &record.cascade_manifest
            && existing != &manifest_records
        {
            return Err(FleetActivationOpsError::EvidenceMismatch);
        }
        if !record.credential_manifests.is_empty()
            && record.credential_manifests != [credential_manifest_record.clone()]
        {
            return Err(FleetActivationOpsError::EvidenceMismatch);
        }

        evidence.cascade = Some(source);
        evidence.credential = Some(credential_record);
        record.cascade_manifest = Some(manifest_records);
        record.credential_manifests = vec![credential_manifest_record];
        replace_record(record)?;
        Self::status(true)
    }

    pub(crate) fn prepare_applied_state_snapshot(
        hash: [u8; 32],
    ) -> Result<PreparedFleetActivationSnapshot, FleetActivationOpsError> {
        prepare_applied_snapshot(Some(hash), None)
    }

    pub(crate) fn prepare_applied_topology_snapshot(
        hash: [u8; 32],
    ) -> Result<PreparedFleetActivationSnapshot, FleetActivationOpsError> {
        prepare_applied_snapshot(None, Some(hash))
    }

    pub(crate) fn commit_prepared_snapshot(prepared: PreparedFleetActivationSnapshot) {
        let Some(record) = prepared.0 else {
            return;
        };
        assert!(
            FleetActivation::replace(record),
            "prepared Fleet activation snapshot requires initialized protected state"
        );
    }

    pub(crate) fn prepare_credential_generation(
        request: FleetCredentialGenerationRequest,
    ) -> Result<FleetActivationStatusResponse, FleetActivationOpsError> {
        let mut record = FleetActivation::get().ok_or(FleetActivationOpsError::NotInitialized)?;
        if record.component_runtime.is_some() {
            return Err(FleetActivationOpsError::InvalidTransition {
                reason: "managed Component runtimes use exact Directory-bound runtime activation"
                    .to_string(),
            });
        }
        let (identity, evidence) = match &mut record.state {
            FleetActivationStateRecord::Prepared {
                identity, evidence, ..
            } => (identity, evidence),
            FleetActivationStateRecord::Active {
                identity, evidence, ..
            } => {
                if identity.operation_id != request.operation_id
                    || evidence.credential.as_ref()
                        != Some(&credential_dto_to_record(request.credential))
                {
                    return Err(FleetActivationOpsError::IdentityMismatch);
                }
                return Self::status(false);
            }
        };
        if identity.operation_id != request.operation_id || request.credential.generation == 0 {
            return Err(FleetActivationOpsError::IdentityMismatch);
        }

        let next = credential_dto_to_record(request.credential);
        match &evidence.credential {
            None if next.generation == 1 => {}
            Some(existing) if existing == &next => return Self::status(false),
            Some(existing) if existing.generation.checked_add(1) == Some(next.generation) => {}
            _ => {
                return Err(FleetActivationOpsError::InvalidTransition {
                    reason: "credential generation must be exact-idempotent or advance by one"
                        .to_string(),
                });
            }
        }
        evidence.credential = Some(next);
        replace_record(record)?;
        Self::status(false)
    }

    pub(crate) fn activate(
        request: FleetActivationRequest,
        is_root: bool,
        activated_at_ns: u64,
    ) -> Result<FleetActivationTransition, FleetActivationOpsError> {
        let mut record = FleetActivation::get().ok_or(FleetActivationOpsError::NotInitialized)?;
        if record.component_runtime.is_some() {
            return Err(FleetActivationOpsError::InvalidTransition {
                reason: "managed Component runtimes use exact Directory-bound runtime activation"
                    .to_string(),
            });
        }
        let (identity, evidence, application_init_args) = match &record.state {
            FleetActivationStateRecord::Prepared {
                identity,
                evidence,
                application_init_args,
            } => (
                identity.clone(),
                evidence.clone(),
                application_init_args.clone(),
            ),
            FleetActivationStateRecord::Active {
                identity, evidence, ..
            } => (identity.clone(), evidence.clone(), None),
        };
        if is_root && application_init_args.is_some() {
            return Err(FleetActivationOpsError::InvalidRecord {
                reason: "root Fleet activation retains non-root application init arguments"
                    .to_string(),
            });
        }
        if identity.operation_id != request.operation_id
            || evidence.credential.as_ref() != Some(&credential_dto_to_record(request.credential))
        {
            return Err(FleetActivationOpsError::IdentityMismatch);
        }
        let cascade =
            evidence
                .cascade
                .clone()
                .ok_or_else(|| FleetActivationOpsError::InvalidTransition {
                    reason: "cascade evidence is incomplete".to_string(),
                })?;
        if is_root != matches!(cascade, FleetCascadeActivationEvidenceRecord::Source { .. }) {
            return Err(FleetActivationOpsError::InvalidTransition {
                reason: "cascade evidence does not match the runtime role".to_string(),
            });
        }
        let expected_hash =
            crate::ops::fleet_activation::FleetActivationEvidenceOps::activation_evidence_hash(
                &identity_record_to_dto(&identity),
                &cascade_record_to_dto(&cascade),
                request.credential,
            )
            .map_err(|error| FleetActivationOpsError::InvalidTransition {
                reason: error.to_string(),
            })?;
        if expected_hash != request.activation_evidence_hash {
            return Err(FleetActivationOpsError::EvidenceMismatch);
        }
        if matches!(record.state, FleetActivationStateRecord::Active { .. }) {
            return Ok(FleetActivationTransition {
                status: Self::status(is_root)?,
                transitioned: false,
                application_init_args: None,
            });
        }

        record.state = FleetActivationStateRecord::Active {
            identity,
            evidence,
            activated_at_ns,
        };
        record.prepared_state_snapshot_hash = None;
        record.prepared_topology_snapshot_hash = None;
        replace_record(record)?;
        Ok(FleetActivationTransition {
            status: Self::status(is_root)?,
            transitioned: true,
            application_init_args,
        })
    }

    #[cfg(test)]
    pub(crate) fn reset_for_tests() {
        FleetActivation::import(FleetActivationData::default());
    }
}

fn initialize_prepared(
    prepared: PreparedFleetActivation,
    component_binding: Option<ManagedCanisterBinding>,
    application_init_args: Option<Vec<u8>>,
) -> Result<FleetActivationIdentity, FleetActivationOpsError> {
    let root_authority = prepared.root_authority.map(root_authority_model_to_record);
    let record = FleetActivationRecord {
        state: FleetActivationStateRecord::Prepared {
            identity: FleetActivationIdentityRecord {
                fleet: prepared.identity.fleet.clone(),
                operation_id: prepared.identity.operation_id,
                release_build_id: prepared.identity.release_build_id,
            },
            evidence: FleetActivationEvidenceRecord {
                cascade: None,
                credential: None,
            },
            application_init_args,
        },
        root_authority,
        prepared_state_snapshot_hash: None,
        prepared_topology_snapshot_hash: None,
        cascade_manifest: None,
        credential_manifests: Vec::new(),
        component_runtime: component_binding.map(|binding| ComponentRuntimeRecord {
            binding,
            directory: None,
            activation: None,
        }),
    };
    validate_record_bound(&record)?;
    if !FleetActivation::initialize(record) {
        return Err(FleetActivationOpsError::AlreadyInitialized);
    }
    Ok(FleetActivationIdentity {
        fleet: prepared.identity.fleet,
        operation_id: prepared.identity.operation_id,
        release_build_id: prepared.identity.release_build_id,
    })
}

fn root_authority_model_to_record(
    authority: PreparedFleetSubnetRootAuthority,
) -> FleetSubnetRootAuthorityRecord {
    FleetSubnetRootAuthorityRecord {
        binding: authority.binding,
        initial_release_set: authority.initial_release_set,
        expected_module_hash: authority.expected_module_hash,
    }
}

fn root_authority_record_to_dto(
    authority: FleetSubnetRootAuthorityRecord,
) -> FleetSubnetRootAuthority {
    FleetSubnetRootAuthority {
        binding: authority.binding,
        initial_release_set: authority.initial_release_set,
        expected_module_hash: authority.expected_module_hash,
    }
}

fn validate_record_bound(record: &FleetActivationRecord) -> Result<(), FleetActivationOpsError> {
    let bytes = crate::cdk::serialize::serialize(record)
        .map_err(|error| FleetActivationOpsError::Encode(error.to_string()))?;
    let maximum = MAX_FLEET_ACTIVATION_RECORD_BYTES as usize;
    if bytes.len() > maximum {
        return Err(FleetActivationOpsError::RecordTooLarge {
            maximum,
            observed: bytes.len(),
        });
    }
    Ok(())
}

fn prepare_applied_snapshot(
    state_hash: Option<[u8; 32]>,
    topology_hash: Option<[u8; 32]>,
) -> Result<PreparedFleetActivationSnapshot, FleetActivationOpsError> {
    let mut record = FleetActivation::get().ok_or(FleetActivationOpsError::NotInitialized)?;
    let FleetActivationStateRecord::Prepared { evidence, .. } = &mut record.state else {
        return Ok(PreparedFleetActivationSnapshot(None));
    };
    if matches!(
        evidence.cascade,
        Some(FleetCascadeActivationEvidenceRecord::Source { .. })
    ) {
        return Err(FleetActivationOpsError::InvalidTransition {
            reason: "non-root applied snapshot cannot replace root source evidence".to_string(),
        });
    }
    if let Some(hash) = state_hash {
        record.prepared_state_snapshot_hash = Some(hash);
    }
    if let Some(hash) = topology_hash {
        record.prepared_topology_snapshot_hash = Some(hash);
    }
    if let (Some(state_snapshot_hash), Some(topology_snapshot_hash)) = (
        record.prepared_state_snapshot_hash,
        record.prepared_topology_snapshot_hash,
    ) {
        evidence.cascade = Some(FleetCascadeActivationEvidenceRecord::Applied {
            state_snapshot_hash,
            topology_snapshot_hash,
        });
    }
    validate_record_bound(&record)?;
    Ok(PreparedFleetActivationSnapshot(Some(record)))
}

fn replace_record(record: FleetActivationRecord) -> Result<(), FleetActivationOpsError> {
    validate_record_bound(&record)?;
    if !FleetActivation::replace(record) {
        return Err(FleetActivationOpsError::NotInitialized);
    }
    Ok(())
}

fn component_runtime_status(
    record: FleetActivationRecord,
) -> Result<ComponentRuntimeStatusResponse, FleetActivationOpsError> {
    let (operation_id, runtime_active, state_activated_at_ns) = match &record.state {
        FleetActivationStateRecord::Prepared { identity, .. } => {
            (identity.operation_id, false, None)
        }
        FleetActivationStateRecord::Active {
            identity,
            activated_at_ns,
            ..
        } => (identity.operation_id, true, Some(*activated_at_ns)),
    };
    let component_runtime =
        record
            .component_runtime
            .ok_or_else(|| FleetActivationOpsError::InvalidRecord {
                reason: "protected non-root is not a managed Component-tree runtime".to_string(),
            })?;
    if let Some(directory) = &component_runtime.directory {
        validate_component_runtime_directory_record(directory)?;
    }
    if let Some(activation) = &component_runtime.activation {
        validate_component_runtime_directory_record(&activation.directory)?;
    }
    let (phase, authority, authority_hash, direct_children_hash, activation) = match (
        runtime_active,
        component_runtime.directory,
        component_runtime.activation,
    ) {
        (false, None, None) => (
            ComponentRuntimePhase::AwaitingDirectory,
            None,
            None,
            None,
            None,
        ),
        (false, Some(directory), None) => (
            ComponentRuntimePhase::DirectoryPrepared,
            Some(component_runtime_directory_record_to_dto(
                &directory.authority,
            )),
            Some(directory.authority_hash),
            Some(directory.direct_children_hash),
            None,
        ),
        (true, Some(directory), Some(activation))
            if Some(activation.activated_at_ns) == state_activated_at_ns =>
        {
            (
                ComponentRuntimePhase::Active,
                Some(component_runtime_directory_record_to_dto(
                    &directory.authority,
                )),
                Some(directory.authority_hash),
                Some(directory.direct_children_hash),
                Some(ComponentRuntimeActivationEvidence {
                    directory_authority_hash: activation.directory.authority_hash,
                    activated_at_ns: activation.activated_at_ns,
                }),
            )
        }
        _ => {
            return Err(FleetActivationOpsError::InvalidRecord {
                reason:
                    "Component runtime phase, Directory authority and activation receipt disagree"
                        .to_string(),
            });
        }
    };
    Ok(ComponentRuntimeStatusResponse {
        operation_id,
        binding: component_runtime.binding,
        phase,
        authority,
        authority_hash,
        direct_children_hash,
        activation,
    })
}

fn component_runtime_preparation_status(
    record: FleetActivationRecord,
) -> Result<ComponentRuntimeStatusResponse, FleetActivationOpsError> {
    let mut status = component_runtime_activation_status(record)?;
    status.phase = ComponentRuntimePhase::DirectoryPrepared;
    status.activation = None;
    Ok(status)
}

fn replay_component_runtime_activation(
    record: FleetActivationRecord,
    request: ComponentRuntimeActivationRequest,
) -> Result<ComponentRuntimeActivationTransition, FleetActivationOpsError> {
    let activation = record
        .component_runtime
        .as_ref()
        .and_then(|runtime| runtime.activation.as_ref())
        .ok_or_else(|| FleetActivationOpsError::InvalidRecord {
            reason: "Active Component runtime has no immutable activation receipt".to_string(),
        })?;
    if activation.directory.authority_hash != request.directory_authority_hash {
        return Err(FleetActivationOpsError::EvidenceMismatch);
    }
    Ok(ComponentRuntimeActivationTransition {
        status: component_runtime_activation_status(record)?,
        transitioned: false,
        application_init_args: None,
    })
}

fn component_runtime_activation_status(
    record: FleetActivationRecord,
) -> Result<ComponentRuntimeStatusResponse, FleetActivationOpsError> {
    let activation_directory = record
        .component_runtime
        .as_ref()
        .and_then(|runtime| runtime.activation.as_ref())
        .ok_or_else(|| FleetActivationOpsError::InvalidRecord {
            reason: "Active Component runtime has no immutable activation receipt".to_string(),
        })?
        .directory
        .clone();
    let mut status = component_runtime_status(record)?;
    status.authority = Some(component_runtime_directory_record_to_dto(
        &activation_directory.authority,
    ));
    status.authority_hash = Some(activation_directory.authority_hash);
    status.direct_children_hash = Some(activation_directory.direct_children_hash);
    Ok(status)
}

fn validate_component_runtime_directory_record(
    directory: &ComponentRuntimeDirectoryRecord,
) -> Result<(), FleetActivationOpsError> {
    let authority = component_runtime_directory_record_to_dto(&directory.authority);
    let authority_hash =
        crate::ops::component_runtime::ComponentRuntimeOps::directory_authority_hash(&authority)
            .map_err(|_| FleetActivationOpsError::InvalidRecord {
                reason: "Component runtime Directory authority cannot be hashed".to_string(),
            })?;
    if authority_hash != directory.authority_hash {
        return Err(FleetActivationOpsError::InvalidRecord {
            reason: "Component runtime Directory authority does not match its retained hash"
                .to_string(),
        });
    }
    if directory.direct_children_hash == [0; 32] {
        return Err(FleetActivationOpsError::InvalidRecord {
            reason: "Component runtime Directory has no direct-child projection hash".to_string(),
        });
    }
    Ok(())
}

fn component_runtime_directory_dto_to_record(
    authority: ComponentRuntimeDirectoryAuthority,
) -> ComponentRuntimeDirectoryAuthorityRecord {
    let ComponentRuntimeDirectoryAuthority { fleet, component } = authority;
    let FleetDirectorySnapshot {
        provenance: fleet_provenance,
        fleet_subnet_roots,
    } = fleet;
    let FleetDirectoryProvenance {
        registry,
        source_fleet_subnet_root: fleet_source,
    } = fleet_provenance;
    let ComponentDirectoryHead {
        provenance: component_provenance,
        descendant_count,
    } = component;
    let ComponentDirectoryProvenance {
        component,
        source_fleet_subnet_root: component_source,
        component_registry_revision,
        component_registry_content_hash,
        synchronized_at_ns,
    } = component_provenance;

    ComponentRuntimeDirectoryAuthorityRecord {
        fleet: FleetDirectorySnapshotRecord {
            provenance: FleetDirectoryProvenanceRecord {
                registry: FleetRegistryVersionRecord {
                    authority: registry.authority,
                    revision: registry.revision,
                    content_hash: registry.content_hash,
                },
                source_fleet_subnet_root: fleet_source,
            },
            fleet_subnet_roots: fleet_subnet_roots
                .into_iter()
                .map(|entry| FleetSubnetRootDirectoryEntryRecord {
                    placement_subnet: entry.placement_subnet,
                    fleet_subnet_root: entry.fleet_subnet_root,
                    status: fleet_subnet_root_status_dto_to_record(entry.status),
                })
                .collect(),
        },
        component: ComponentDirectoryHeadRecord {
            provenance: ComponentDirectoryProvenanceRecord {
                component,
                source_fleet_subnet_root: component_source,
                component_registry_revision,
                component_registry_content_hash,
                synchronized_at_ns,
            },
            descendant_count,
        },
    }
}

fn component_runtime_directory_record_to_dto(
    authority: &ComponentRuntimeDirectoryAuthorityRecord,
) -> ComponentRuntimeDirectoryAuthority {
    ComponentRuntimeDirectoryAuthority {
        fleet: FleetDirectorySnapshot {
            provenance: FleetDirectoryProvenance {
                registry: FleetRegistryVersion {
                    authority: authority.fleet.provenance.registry.authority.clone(),
                    revision: authority.fleet.provenance.registry.revision,
                    content_hash: authority.fleet.provenance.registry.content_hash,
                },
                source_fleet_subnet_root: authority.fleet.provenance.source_fleet_subnet_root,
            },
            fleet_subnet_roots: authority
                .fleet
                .fleet_subnet_roots
                .iter()
                .map(|entry| FleetSubnetRootDirectoryEntry {
                    placement_subnet: entry.placement_subnet,
                    fleet_subnet_root: entry.fleet_subnet_root,
                    status: fleet_subnet_root_status_record_to_dto(entry.status),
                })
                .collect(),
        },
        component: ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: authority.component.provenance.component.clone(),
                source_fleet_subnet_root: authority.component.provenance.source_fleet_subnet_root,
                component_registry_revision: authority
                    .component
                    .provenance
                    .component_registry_revision,
                component_registry_content_hash: authority
                    .component
                    .provenance
                    .component_registry_content_hash,
                synchronized_at_ns: authority.component.provenance.synchronized_at_ns,
            },
            descendant_count: authority.component.descendant_count,
        },
    }
}

const fn fleet_subnet_root_status_dto_to_record(
    status: FleetSubnetRootStatus,
) -> FleetSubnetRootStatusRecord {
    match status {
        FleetSubnetRootStatus::Joining => FleetSubnetRootStatusRecord::Joining,
        FleetSubnetRootStatus::Active => FleetSubnetRootStatusRecord::Active,
        FleetSubnetRootStatus::Draining => FleetSubnetRootStatusRecord::Draining,
        FleetSubnetRootStatus::Removed => FleetSubnetRootStatusRecord::Removed,
    }
}

const fn fleet_subnet_root_status_record_to_dto(
    status: FleetSubnetRootStatusRecord,
) -> FleetSubnetRootStatus {
    match status {
        FleetSubnetRootStatusRecord::Joining => FleetSubnetRootStatus::Joining,
        FleetSubnetRootStatusRecord::Active => FleetSubnetRootStatus::Active,
        FleetSubnetRootStatusRecord::Draining => FleetSubnetRootStatus::Draining,
        FleetSubnetRootStatusRecord::Removed => FleetSubnetRootStatus::Removed,
    }
}

const fn credential_dto_to_record(
    credential: FleetCredentialGenerationRef,
) -> FleetCredentialGenerationRefRecord {
    FleetCredentialGenerationRefRecord {
        generation: credential.generation,
        manifest_hash: credential.manifest_hash,
    }
}

fn credential_manifest_to_record(
    manifest: FleetCredentialManifest,
) -> FleetCredentialManifestRecord {
    FleetCredentialManifestRecord {
        fleet: manifest.fleet,
        activation_id: manifest.activation_id,
        generation: manifest.generation,
        root_policy_set_hash: manifest.root_policy_set_hash,
        renewal_template_set_hash: manifest.renewal_template_set_hash,
        entries: manifest
            .entries
            .into_iter()
            .map(|entry| FleetCredentialManifestEntryRecord {
                root_issuer: entry.root_issuer,
                subject_canister: entry.subject_canister,
                not_before_ns: entry.not_before_ns,
                expires_at_ns: entry.expires_at_ns,
                key_identity_hash: entry.key_identity_hash,
                cert_hash: entry.cert_hash,
                proof_hash: entry.proof_hash,
                bundle_hash: entry.bundle_hash,
            })
            .collect(),
    }
}

fn identity_record_to_dto(record: &FleetActivationIdentityRecord) -> FleetActivationIdentity {
    FleetActivationIdentity {
        fleet: record.fleet.clone(),
        operation_id: record.operation_id,
        release_build_id: record.release_build_id,
    }
}

const fn cascade_record_to_dto(
    record: &FleetCascadeActivationEvidenceRecord,
) -> FleetCascadeActivationEvidence {
    match record {
        FleetCascadeActivationEvidenceRecord::Source {
            cascade_manifest_hash,
        } => FleetCascadeActivationEvidence::Source {
            cascade_manifest_hash: *cascade_manifest_hash,
        },
        FleetCascadeActivationEvidenceRecord::Applied {
            state_snapshot_hash,
            topology_snapshot_hash,
        } => FleetCascadeActivationEvidence::Applied {
            state_snapshot_hash: *state_snapshot_hash,
            topology_snapshot_hash: *topology_snapshot_hash,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Cycles,
        config::{ComponentLimits, ComponentSpec},
        dto::{
            component_registry::{
                ComponentDirectoryHead, ComponentDirectoryProvenance,
                ComponentRuntimeDirectoryAuthority, ComponentRuntimeDirectoryPreparationRequest,
                ComponentRuntimePhase,
            },
            fleet_registry::{
                FleetDirectoryProvenance, FleetDirectorySnapshot, FleetRegistryVersion,
                FleetSubnetRootDirectoryEntry, FleetSubnetRootStatus,
            },
            fleet_subnet_root::{FleetSubnetRootAuthority, FleetSubnetRootInitArgs},
        },
        ids::{
            AppId, CanisterRole, CanonicalNetworkId, ComponentBinding, ComponentInstanceId,
            ComponentSpecAdmission, CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding,
            FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetRootBinding,
            FleetSubnetRootLimits, FleetSubnetRootReleaseSet, ManagedCanisterBinding,
            ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
        },
        storage::stable::fleet_activation::{
            FleetActivationEvidenceRecord, FleetActivationStateRecord,
            FleetCascadeActivationEvidenceRecord, FleetCredentialGenerationRefRecord,
            FleetCredentialManifestRecord,
        },
    };
    use candid::Principal;

    fn release_build(byte: u8) -> ReleaseBuildId {
        ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([byte; 32]))
    }

    fn input(release_build_id: ReleaseBuildId) -> FleetSubnetRootInitArgs {
        let component_spec = "projects".parse().expect("Component Spec");
        let spec_hash = [10; 32];
        let topology = topology();
        let admissions = vec![ComponentSpecAdmission {
            component_spec,
            spec_hash,
            maximum_root_instances: 2,
        }];
        let projection = topology
            .project_for_admissions(&admissions)
            .expect("root topology projection");
        FleetSubnetRootInitArgs {
            authority: FleetSubnetRootAuthority {
                binding: FleetSubnetRootBinding {
                    authority: FleetRegistryAuthority {
                        binding: FleetCoordinatorBinding {
                            fleet: FleetBinding {
                                fleet: FleetKey {
                                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                                    fleet_id: FleetId::from_generated_bytes([11; 32]),
                                },
                                app: AppId::from("toko"),
                            },
                            coordinator_subnet: SubnetId::from_principal(Principal::from_slice(
                                &[20; 29],
                            )),
                            coordinator: Principal::from_slice(&[21; 29]),
                        },
                        epoch: 1,
                    },
                    placement_subnet: SubnetId::from_principal(Principal::from_slice(&[22; 29])),
                    fleet_subnet_root: Principal::from_slice(&[23; 29]),
                    component_admissions: admissions,
                    component_topology_digest: projection.digest().expect("topology digest"),
                    limits: FleetSubnetRootLimits {
                        maximum_component_instances: 10,
                        maximum_managed_canisters: 1_000,
                        maximum_registry_bytes: 4_194_304,
                        maximum_wasm_store_bytes: 40_000_000,
                        canister_pool: crate::ids::FleetSubnetCanisterPoolConfig {
                            minimum_size: 1,
                            maximum_size: 10,
                            canister_cycles: Cycles::new(5_000_000_000_000),
                        },
                        cycles_funding: CyclesFundingBudget {
                            window_secs: 3_600,
                            maximum_cycles: Cycles::new(1_000_000_000_000),
                        },
                    },
                },
                initial_release_set: FleetSubnetRootReleaseSet {
                    release_build_id,
                    manifest_digest: ReleaseSetDigest::from_bytes([24; 32]),
                },
                expected_module_hash: [13; 32],
            },
            install_id: [12; 32],
            canister_pool_imports: Vec::new(),
        }
    }

    fn topology() -> ComponentTopology {
        ComponentTopology {
            component_specs: vec![ComponentSpec {
                component_spec: "projects".parse().expect("Component Spec"),
                spec_hash: [10; 32],
                component_role: CanisterRole::from("project_hub"),
                maximum_fleet_instances: 10,
                limits: ComponentLimits {
                    maximum_descendants: 100,
                    maximum_registry_bytes: 1_048_576,
                    cycles_funding: CyclesFundingBudget {
                        window_secs: 3_600,
                        maximum_cycles: Cycles::new(1_000_000_000_000),
                    },
                },
                children: Vec::new(),
                spawn_grants: Vec::new(),
            }],
            provisioning_grants: Vec::new(),
        }
    }

    fn initialize_root(
        input: FleetSubnetRootInitArgs,
        embedded_release_build_id: ReleaseBuildId,
    ) -> Result<FleetActivationIdentity, FleetActivationOpsError> {
        let root_canister = input.authority.binding.fleet_subnet_root;
        FleetActivationOps::initialize_root_prepared(
            input,
            embedded_release_build_id,
            &AppId::from("toko"),
            &topology(),
            root_canister,
        )
    }

    fn initialize_nonroot(
        input: FleetSubnetRootInitArgs,
        embedded_release_build_id: ReleaseBuildId,
        application_init_args: Option<Vec<u8>>,
    ) -> Result<FleetActivationIdentity, FleetActivationOpsError> {
        FleetActivationOps::initialize_nonroot_prepared(
            input.authority.binding.authority.binding.fleet,
            input.install_id,
            input.authority.initial_release_set.release_build_id,
            embedded_release_build_id,
            None,
            application_init_args,
        )
    }

    fn record_state_snapshot(hash: [u8; 32]) {
        let prepared = FleetActivationOps::prepare_applied_state_snapshot(hash)
            .expect("prepare state evidence");
        FleetActivationOps::commit_prepared_snapshot(prepared);
    }

    fn record_topology_snapshot(hash: [u8; 32]) {
        let prepared = FleetActivationOps::prepare_applied_topology_snapshot(hash)
            .expect("prepare topology evidence");
        FleetActivationOps::commit_prepared_snapshot(prepared);
    }

    #[test]
    fn root_init_commits_exact_prepared_identity_once() {
        FleetActivationOps::reset_for_tests();
        let release_build_id = release_build(14);
        let identity = initialize_root(input(release_build_id), release_build_id)
            .expect("initialize Prepared");
        let stored = FleetActivationOps::snapshot()
            .record
            .expect("protected activation record");

        assert_eq!(identity.operation_id, [12; 32]);
        let FleetActivationStateRecord::Prepared {
            identity: stored_identity,
            evidence:
                FleetActivationEvidenceRecord {
                    cascade: None,
                    credential: None,
                },
            application_init_args: None,
        } = stored.state
        else {
            panic!("root init must store an empty Prepared state")
        };
        assert_eq!(stored_identity.operation_id, [12; 32]);
        assert!(matches!(
            initialize_root(input(release_build_id), release_build_id)
                .expect_err("second initialization must fail"),
            FleetActivationOpsError::AlreadyInitialized
        ));

        FleetActivationOps::reset_for_tests();
    }

    #[test]
    fn root_init_mismatch_writes_no_activation_record() {
        FleetActivationOps::reset_for_tests();
        let supplied = release_build(15);
        let embedded = release_build(16);

        assert!(matches!(
            initialize_root(input(supplied), embedded),
            Err(FleetActivationOpsError::Admission(
                PrepareFleetActivationError::ReleaseBuildMismatch { .. }
            ))
        ));
        assert_eq!(
            FleetActivationOps::snapshot(),
            FleetActivationData::default()
        );
    }

    #[test]
    fn nonroot_init_commits_identity_empty_evidence_and_application_args_once() {
        FleetActivationOps::reset_for_tests();
        let release_build_id = release_build(32);
        let root_input = input(release_build_id);
        let identity = initialize_nonroot(root_input, release_build_id, Some(vec![33, 34]))
            .expect("initialize non-root Prepared");
        let stored = FleetActivationOps::snapshot()
            .record
            .expect("protected activation record");

        assert_eq!(identity.operation_id, [12; 32]);
        assert!(matches!(
            stored.state,
            FleetActivationStateRecord::Prepared {
                evidence: FleetActivationEvidenceRecord {
                    cascade: None,
                    credential: None,
                },
                application_init_args: Some(ref args),
                ..
            } if args == &[33, 34]
        ));

        FleetActivationOps::reset_for_tests();
    }

    #[test]
    fn nonroot_init_mismatch_writes_no_activation_record() {
        FleetActivationOps::reset_for_tests();
        let supplied = release_build(33);
        let embedded = release_build(34);
        let root_input = input(supplied);

        assert!(matches!(
            initialize_nonroot(root_input, embedded, None),
            Err(FleetActivationOpsError::Admission(
                PrepareFleetActivationError::ReleaseBuildMismatch { .. }
            ))
        ));
        assert_eq!(
            FleetActivationOps::snapshot(),
            FleetActivationData::default()
        );
    }

    #[test]
    fn component_directory_preparation_is_exact_idempotent_and_remains_prepared() {
        FleetActivationOps::reset_for_tests();
        let release_build_id = release_build(35);
        let root_input = input(release_build_id);
        let root = root_input.authority.binding.clone();
        let binding = ComponentBinding {
            authority: root.authority.clone(),
            component: ComponentInstanceId::from_generated_bytes([36; 32]),
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [10; 32],
            role: CanisterRole::from("project_hub"),
            placement_subnet: root.placement_subnet,
            fleet_subnet_root: root.fleet_subnet_root,
            canister_id: Principal::from_slice(&[37; 29]),
        };
        FleetActivationOps::initialize_nonroot_prepared(
            root.authority.binding.fleet.clone(),
            root_input.install_id,
            release_build_id,
            release_build_id,
            Some(ManagedCanisterBinding::Component(binding.clone())),
            None,
        )
        .expect("initialize Component runtime");
        let awaiting =
            FleetActivationOps::component_runtime_status().expect("awaiting Directory status");
        assert_eq!(awaiting.phase, ComponentRuntimePhase::AwaitingDirectory);
        assert_eq!(awaiting.authority, None);

        let authority = ComponentRuntimeDirectoryAuthority {
            fleet: FleetDirectorySnapshot {
                provenance: FleetDirectoryProvenance {
                    registry: FleetRegistryVersion {
                        authority: root.authority,
                        revision: 3,
                        content_hash: [38; 32],
                    },
                    source_fleet_subnet_root: root.fleet_subnet_root,
                },
                fleet_subnet_roots: vec![FleetSubnetRootDirectoryEntry {
                    placement_subnet: root.placement_subnet,
                    fleet_subnet_root: root.fleet_subnet_root,
                    status: FleetSubnetRootStatus::Active,
                }],
            },
            component: ComponentDirectoryHead {
                provenance: ComponentDirectoryProvenance {
                    component: binding,
                    source_fleet_subnet_root: root.fleet_subnet_root,
                    component_registry_revision: 1,
                    component_registry_content_hash: [39; 32],
                    synchronized_at_ns: 40,
                },
                descendant_count: 0,
            },
        };
        let authority_hash =
            crate::ops::component_runtime::ComponentRuntimeOps::directory_authority_hash(
                &authority,
            )
            .expect("Directory authority hash");
        let request = ComponentRuntimeDirectoryPreparationRequest {
            operation_id: root_input.install_id,
            authority,
            direct_children: Vec::new(),
        };
        let expected_authority = request.authority.clone();
        let prepared = FleetActivationOps::prepare_component_runtime_directory(
            request.clone(),
            authority_hash,
            [60; 32],
        )
        .expect("prepare Directory");
        let repeated = FleetActivationOps::prepare_component_runtime_directory(
            request,
            authority_hash,
            [60; 32],
        )
        .expect("repeat Directory preparation");
        assert_eq!(repeated, prepared);
        assert_eq!(prepared.phase, ComponentRuntimePhase::DirectoryPrepared);
        assert_eq!(prepared.authority, Some(expected_authority));
        assert_eq!(prepared.authority_hash, Some(authority_hash));
        assert_eq!(prepared.activation, None);
        assert_eq!(
            FleetActivationOps::status(false)
                .expect("Fleet activation status")
                .phase,
            crate::dto::fleet_activation::FleetActivationPhase::Prepared
        );
        FleetActivationOps::reset_for_tests();
    }

    #[test]
    fn component_runtime_activation_is_directory_bound_and_exact_idempotent() {
        FleetActivationOps::reset_for_tests();
        let release_build_id = release_build(41);
        let root_input = input(release_build_id);
        let root = root_input.authority.binding.clone();
        let binding = ComponentBinding {
            authority: root.authority.clone(),
            component: ComponentInstanceId::from_generated_bytes([42; 32]),
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [43; 32],
            role: CanisterRole::from("project_hub"),
            placement_subnet: root.placement_subnet,
            fleet_subnet_root: root.fleet_subnet_root,
            canister_id: Principal::from_slice(&[44; 29]),
        };
        FleetActivationOps::initialize_nonroot_prepared(
            root.authority.binding.fleet.clone(),
            root_input.install_id,
            release_build_id,
            release_build_id,
            Some(ManagedCanisterBinding::Component(binding.clone())),
            Some(vec![45, 46]),
        )
        .expect("initialize Component runtime");
        let authority = ComponentRuntimeDirectoryAuthority {
            fleet: FleetDirectorySnapshot {
                provenance: FleetDirectoryProvenance {
                    registry: FleetRegistryVersion {
                        authority: root.authority,
                        revision: 3,
                        content_hash: [47; 32],
                    },
                    source_fleet_subnet_root: root.fleet_subnet_root,
                },
                fleet_subnet_roots: vec![FleetSubnetRootDirectoryEntry {
                    placement_subnet: root.placement_subnet,
                    fleet_subnet_root: root.fleet_subnet_root,
                    status: FleetSubnetRootStatus::Active,
                }],
            },
            component: ComponentDirectoryHead {
                provenance: ComponentDirectoryProvenance {
                    component: binding,
                    source_fleet_subnet_root: root.fleet_subnet_root,
                    component_registry_revision: 1,
                    component_registry_content_hash: [48; 32],
                    synchronized_at_ns: 49,
                },
                descendant_count: 0,
            },
        };
        let authority_hash =
            crate::ops::component_runtime::ComponentRuntimeOps::directory_authority_hash(
                &authority,
            )
            .expect("Directory authority hash");
        FleetActivationOps::prepare_component_runtime_directory(
            ComponentRuntimeDirectoryPreparationRequest {
                operation_id: root_input.install_id,
                authority: authority.clone(),
                direct_children: Vec::new(),
            },
            authority_hash,
            [60; 32],
        )
        .expect("prepare Directory");
        let request = ComponentRuntimeActivationRequest {
            operation_id: root_input.install_id,
            directory_authority_hash: authority_hash,
        };
        let activated = FleetActivationOps::activate_component_runtime(request, 50)
            .expect("activate Component runtime");
        let repeated = FleetActivationOps::activate_component_runtime(request, 51)
            .expect("repeat Component runtime activation");

        assert_component_runtime_activation_transition(
            &activated,
            &repeated,
            authority_hash,
            request,
        );

        assert_component_runtime_directory_progression(
            root_input.install_id,
            authority,
            authority_hash,
            request,
            &activated.status,
        );

        assert_component_runtime_directory_corruption_fails_closed(FleetActivationOps::snapshot());

        FleetActivationOps::reset_for_tests();
    }

    fn assert_component_runtime_activation_transition(
        activated: &ComponentRuntimeActivationTransition,
        repeated: &ComponentRuntimeActivationTransition,
        authority_hash: [u8; 32],
        request: ComponentRuntimeActivationRequest,
    ) {
        assert!(activated.transitioned);
        assert_eq!(activated.application_init_args, Some(vec![45, 46]));
        assert!(!repeated.transitioned);
        assert_eq!(repeated.application_init_args, None);
        assert_eq!(repeated.status, activated.status);
        assert_eq!(activated.status.phase, ComponentRuntimePhase::Active);
        assert_eq!(
            activated.status.activation,
            Some(ComponentRuntimeActivationEvidence {
                directory_authority_hash: authority_hash,
                activated_at_ns: 50,
            })
        );
        assert!(matches!(
            FleetActivationOps::activate_component_runtime(
                ComponentRuntimeActivationRequest {
                    directory_authority_hash: [52; 32],
                    ..request
                },
                53,
            ),
            Err(FleetActivationOpsError::EvidenceMismatch)
        ));
    }

    fn assert_component_runtime_directory_progression(
        operation_id: [u8; 32],
        mut active_authority: ComponentRuntimeDirectoryAuthority,
        prepared_authority_hash: [u8; 32],
        activation_request: ComponentRuntimeActivationRequest,
        activated_status: &ComponentRuntimeStatusResponse,
    ) {
        active_authority
            .component
            .provenance
            .component_registry_revision = 3;
        active_authority
            .component
            .provenance
            .component_registry_content_hash = [54; 32];
        active_authority.component.provenance.synchronized_at_ns = 55;
        let active_authority_hash =
            crate::ops::component_runtime::ComponentRuntimeOps::directory_authority_hash(
                &active_authority,
            )
            .expect("active Directory authority hash");
        let synchronization_request = ComponentRuntimeDirectorySynchronizationRequest {
            operation_id,
            authority: active_authority,
            direct_children: Vec::new(),
        };
        let synchronized = FleetActivationOps::synchronize_component_runtime_directory(
            synchronization_request.clone(),
            active_authority_hash,
            [61; 32],
        )
        .expect("synchronize current Directory");
        let synchronized_again = FleetActivationOps::synchronize_component_runtime_directory(
            synchronization_request,
            active_authority_hash,
            [61; 32],
        )
        .expect("repeat current Directory synchronization");
        assert_eq!(synchronized_again, synchronized);
        assert_eq!(synchronized.authority_hash, Some(active_authority_hash));
        assert_eq!(synchronized.activation, activated_status.activation);

        let activation_again =
            FleetActivationOps::activate_component_runtime(activation_request, 56)
                .expect("activation retry after current Directory progression");
        assert_eq!(&activation_again.status, activated_status);
        let preparation_again = FleetActivationOps::prepare_component_runtime_directory(
            ComponentRuntimeDirectoryPreparationRequest {
                operation_id,
                authority: activated_status
                    .authority
                    .clone()
                    .expect("prepared activation authority"),
                direct_children: Vec::new(),
            },
            prepared_authority_hash,
            [60; 32],
        )
        .expect("Directory preparation retry after current Directory progression");
        assert_eq!(
            preparation_again.phase,
            ComponentRuntimePhase::DirectoryPrepared
        );
        assert_eq!(
            preparation_again.authority_hash,
            Some(prepared_authority_hash)
        );
        assert_eq!(preparation_again.activation, None);
    }

    fn assert_component_runtime_directory_corruption_fails_closed(valid: FleetActivationData) {
        let mut corrupted_activation = valid.clone();
        corrupted_activation
            .record
            .as_mut()
            .expect("activation record")
            .component_runtime
            .as_mut()
            .expect("Component runtime")
            .activation
            .as_mut()
            .expect("activation evidence")
            .directory
            .authority_hash = [57; 32];
        FleetActivation::import(corrupted_activation);
        assert!(matches!(
            FleetActivationOps::component_runtime_status(),
            Err(FleetActivationOpsError::InvalidRecord { .. })
        ));
        assert!(matches!(
            FleetActivationOps::status(false),
            Err(FleetActivationOpsError::InvalidRecord { .. })
        ));

        let mut corrupted_current = valid;
        corrupted_current
            .record
            .as_mut()
            .expect("activation record")
            .component_runtime
            .as_mut()
            .expect("Component runtime")
            .directory
            .as_mut()
            .expect("current Directory")
            .authority_hash = [58; 32];
        FleetActivation::import(corrupted_current);
        assert!(matches!(
            FleetActivationOps::component_runtime_status(),
            Err(FleetActivationOpsError::InvalidRecord { .. })
        ));
        assert!(matches!(
            FleetActivationOps::status(false),
            Err(FleetActivationOpsError::InvalidRecord { .. })
        ));
    }

    #[test]
    fn status_projects_the_exact_prepared_identity() {
        FleetActivationOps::reset_for_tests();
        let release_build_id = release_build(17);
        initialize_root(input(release_build_id), release_build_id).expect("initialize Prepared");

        let status = FleetActivationOps::status(true).expect("activation status");

        assert_eq!(
            status.phase,
            crate::dto::fleet_activation::FleetActivationPhase::Prepared
        );
        assert_eq!(status.identity.operation_id, [12; 32]);
        assert_eq!(status.identity.release_build_id, release_build_id);
        assert_eq!(status.cascade, None);
        assert_eq!(status.credential, None);
        assert_eq!(status.activated_at_ns, None);
        assert_eq!(
            FleetActivationOps::require_active(true),
            Err(FleetActivationOpsError::NotActive)
        );

        FleetActivationOps::reset_for_tests();
    }

    #[test]
    fn status_rejects_absent_and_contradictory_protected_state() {
        FleetActivationOps::reset_for_tests();
        assert_eq!(
            FleetActivationOps::status(true),
            Err(FleetActivationOpsError::NotInitialized)
        );

        let release_build_id = release_build(18);
        initialize_root(input(release_build_id), release_build_id).expect("initialize Prepared");
        let mut data = FleetActivationOps::snapshot();
        data.record
            .as_mut()
            .expect("record")
            .credential_manifests
            .push(FleetCredentialManifestRecord {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([11; 32]),
                },
                activation_id: [12; 32],
                generation: 1,
                root_policy_set_hash: [19; 32],
                renewal_template_set_hash: [20; 32],
                entries: Vec::new(),
            });
        FleetActivation::import(data);

        assert!(matches!(
            FleetActivationOps::status(true),
            Err(FleetActivationOpsError::InvalidRecord { .. })
        ));
        assert!(matches!(
            FleetActivationOps::status(false),
            Err(FleetActivationOpsError::InvalidRecord { .. })
        ));

        FleetActivationOps::reset_for_tests();
    }

    #[test]
    fn root_status_rejects_nonroot_application_init_arguments() {
        FleetActivationOps::reset_for_tests();
        let release_build_id = release_build(20);
        initialize_root(input(release_build_id), release_build_id).expect("initialize Prepared");
        let mut data = FleetActivationOps::snapshot();
        let FleetActivationStateRecord::Prepared {
            application_init_args,
            ..
        } = &mut data.record.as_mut().expect("record").state
        else {
            panic!("expected Prepared")
        };
        *application_init_args = Some(vec![21]);
        FleetActivation::import(data);

        assert!(matches!(
            FleetActivationOps::status(true),
            Err(FleetActivationOpsError::InvalidRecord { .. })
        ));

        FleetActivationOps::reset_for_tests();
    }

    #[test]
    fn status_projects_complete_active_root_evidence() {
        FleetActivationOps::reset_for_tests();
        let release_build_id = release_build(21);
        initialize_root(input(release_build_id), release_build_id).expect("initialize Prepared");
        let mut data = FleetActivationOps::snapshot();
        let record = data.record.as_mut().expect("record");
        let FleetActivationStateRecord::Prepared { identity, .. } = &record.state else {
            panic!("expected Prepared")
        };
        let identity = identity.clone();
        record.state = FleetActivationStateRecord::Active {
            identity: identity.clone(),
            evidence: FleetActivationEvidenceRecord {
                cascade: Some(FleetCascadeActivationEvidenceRecord::Source {
                    cascade_manifest_hash: [22; 32],
                }),
                credential: Some(FleetCredentialGenerationRefRecord {
                    generation: 1,
                    manifest_hash: [23; 32],
                }),
            },
            activated_at_ns: 24,
        };
        record.cascade_manifest = Some(Vec::new());
        record.credential_manifests = vec![FleetCredentialManifestRecord {
            fleet: identity.fleet.fleet,
            activation_id: identity.operation_id,
            generation: 1,
            root_policy_set_hash: [25; 32],
            renewal_template_set_hash: [26; 32],
            entries: Vec::new(),
        }];
        FleetActivation::import(data);

        let status = FleetActivationOps::status(true).expect("active root status");

        assert_eq!(
            status.phase,
            crate::dto::fleet_activation::FleetActivationPhase::Active
        );
        assert_eq!(status.activated_at_ns, Some(24));
        assert_eq!(status.cascade_manifest, Some(Vec::new()));
        assert_eq!(
            status
                .credential_manifest
                .as_ref()
                .map(|manifest| manifest.generation),
            Some(1)
        );
        FleetActivationOps::require_active(true).expect("complete Active root");

        FleetActivationOps::reset_for_tests();
    }

    #[test]
    fn status_projects_only_nonroot_applied_evidence() {
        FleetActivationOps::reset_for_tests();
        let release_build_id = release_build(27);
        initialize_nonroot(input(release_build_id), release_build_id, None)
            .expect("initialize Prepared");
        let mut data = FleetActivationOps::snapshot();
        let record = data.record.as_mut().expect("record");
        let FleetActivationStateRecord::Prepared { identity, .. } = &record.state else {
            panic!("expected Prepared")
        };
        record.state = FleetActivationStateRecord::Active {
            identity: identity.clone(),
            evidence: FleetActivationEvidenceRecord {
                cascade: Some(FleetCascadeActivationEvidenceRecord::Applied {
                    state_snapshot_hash: [28; 32],
                    topology_snapshot_hash: [29; 32],
                }),
                credential: Some(FleetCredentialGenerationRefRecord {
                    generation: 1,
                    manifest_hash: [30; 32],
                }),
            },
            activated_at_ns: 31,
        };
        FleetActivation::import(data);

        let status = FleetActivationOps::status(false).expect("active non-root status");

        assert_eq!(
            status.cascade,
            Some(
                crate::dto::fleet_activation::FleetCascadeActivationEvidence::Applied {
                    state_snapshot_hash: [28; 32],
                    topology_snapshot_hash: [29; 32],
                }
            )
        );
        assert_eq!(
            status.credential,
            Some(crate::dto::fleet_activation::FleetCredentialGenerationRef {
                generation: 1,
                manifest_hash: [30; 32],
            })
        );
        assert_eq!(status.cascade_manifest, None);
        assert_eq!(status.credential_manifest, None);

        FleetActivationOps::reset_for_tests();
    }

    #[test]
    fn nonroot_activation_commits_once_and_exact_replay_is_observational() {
        FleetActivationOps::reset_for_tests();
        let release_build_id = release_build(35);
        let root_input = input(release_build_id);
        initialize_nonroot(root_input, release_build_id, Some(vec![35, 36]))
            .expect("initialize non-root Prepared");
        record_state_snapshot([36; 32]);
        record_topology_snapshot([37; 32]);
        let credential = FleetCredentialGenerationRef {
            generation: 1,
            manifest_hash: [38; 32],
        };
        FleetActivationOps::prepare_credential_generation(FleetCredentialGenerationRequest {
            operation_id: [12; 32],
            credential,
        })
        .expect("prepare credential generation");

        let prepared = FleetActivationOps::status(false).expect("prepared status");
        let activation_evidence_hash =
            crate::ops::fleet_activation::FleetActivationEvidenceOps::activation_evidence_hash(
                &prepared.identity,
                prepared.cascade.as_ref().expect("applied cascade evidence"),
                credential,
            )
            .expect("hash activation evidence");
        let request = FleetActivationRequest {
            operation_id: [12; 32],
            credential,
            activation_evidence_hash,
        };

        let first =
            FleetActivationOps::activate(request, false, 39).expect("activate non-root once");
        assert!(first.transitioned);
        assert_eq!(
            first.status.phase,
            crate::dto::fleet_activation::FleetActivationPhase::Active
        );
        assert_eq!(first.status.activated_at_ns, Some(39));
        assert_eq!(first.application_init_args, Some(vec![35, 36]));

        let replay =
            FleetActivationOps::activate(request, false, 40).expect("replay exact activation");
        assert!(!replay.transitioned);
        assert_eq!(replay.status.activated_at_ns, Some(39));
        assert_eq!(replay.application_init_args, None);

        FleetActivationOps::reset_for_tests();
    }

    #[test]
    fn nonroot_activation_rejects_mismatched_evidence_without_transition() {
        FleetActivationOps::reset_for_tests();
        let release_build_id = release_build(41);
        let root_input = input(release_build_id);
        initialize_nonroot(root_input, release_build_id, None)
            .expect("initialize non-root Prepared");
        record_state_snapshot([42; 32]);
        record_topology_snapshot([43; 32]);
        let credential = FleetCredentialGenerationRef {
            generation: 1,
            manifest_hash: [44; 32],
        };
        FleetActivationOps::prepare_credential_generation(FleetCredentialGenerationRequest {
            operation_id: [12; 32],
            credential,
        })
        .expect("prepare credential generation");

        assert_eq!(
            FleetActivationOps::activate(
                FleetActivationRequest {
                    operation_id: [12; 32],
                    credential,
                    activation_evidence_hash: [45; 32],
                },
                false,
                46,
            ),
            Err(FleetActivationOpsError::EvidenceMismatch)
        );
        assert_eq!(
            FleetActivationOps::status(false)
                .expect("status remains readable")
                .phase,
            crate::dto::fleet_activation::FleetActivationPhase::Prepared
        );

        FleetActivationOps::reset_for_tests();
    }
}
