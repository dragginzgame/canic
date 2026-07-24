//! Module: ops::storage::fleet_activation
//!
//! Responsibility: validate, initialize, and project the protected Fleet activation record.
//! Does not own: lifecycle orchestration, embedded build lookup, endpoint policy, or timers.
//! Boundary: initialization writes `Prepared` once; status rejects invalid role/state projections.

mod mapper;

use crate::{
    dto::fleet_activation::{
        CurrentRootInstallIdentity, FleetActivationIdentity, FleetActivationRequest,
        FleetActivationStatusResponse, FleetCascadeActivationEvidence, FleetCascadeManifestEntry,
        FleetCredentialGenerationRef, FleetCredentialGenerationRequest, FleetCredentialManifest,
    },
    ids::{FleetBinding, ReleaseBuildId},
    model::fleet_activation::{
        NonrootInstallIdentity, PrepareFleetActivationError, PreparedFleetActivation,
        RootInstallIdentity, prepare_nonroot_install, prepare_root_install,
    },
    storage::stable::fleet_activation::{
        FleetActivation, FleetActivationData, FleetActivationEvidenceRecord,
        FleetActivationIdentityRecord, FleetActivationRecord, FleetActivationStateRecord,
        FleetCascadeActivationEvidenceRecord, FleetCascadeManifestEntryRecord,
        FleetCredentialGenerationRefRecord, FleetCredentialManifestEntryRecord,
        FleetCredentialManifestRecord, MAX_FLEET_ACTIVATION_RECORD_BYTES,
    },
    view::fleet_activation::FleetActivationTransition,
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
        input: CurrentRootInstallIdentity,
        embedded_release_build_id: ReleaseBuildId,
    ) -> Result<FleetActivationIdentity, FleetActivationOpsError> {
        let prepared = prepare_root_install(
            RootInstallIdentity {
                fleet: input.fleet,
                install_id: input.install_id,
                release_build_id: input.release_build_id,
            },
            embedded_release_build_id,
        )?;
        initialize_prepared(prepared, None)
    }

    pub(crate) fn initialize_nonroot_prepared(
        fleet: FleetBinding,
        install_id: [u8; 32],
        release_build_id: ReleaseBuildId,
        embedded_release_build_id: ReleaseBuildId,
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
        initialize_prepared(prepared, application_init_args)
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the activation snapshot remains staged for lifecycle persistence"
        )
    )]
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
        if credential.generation == 0
            || credential_manifest.fleet != identity.fleet.fleet
            || credential_manifest.activation_id != identity.operation_id
            || credential_manifest.generation != credential.generation
        {
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

    #[cfg(test)]
    pub(crate) fn has_partial_snapshot_evidence_for_tests() -> bool {
        FleetActivation::get().is_some_and(|record| {
            record.prepared_state_snapshot_hash.is_some()
                || record.prepared_topology_snapshot_hash.is_some()
        })
    }
}

fn initialize_prepared(
    prepared: PreparedFleetActivation,
    application_init_args: Option<Vec<u8>>,
) -> Result<FleetActivationIdentity, FleetActivationOpsError> {
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
        prepared_state_snapshot_hash: None,
        prepared_topology_snapshot_hash: None,
        cascade_manifest: None,
        credential_manifests: Vec::new(),
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
        ids::{AppId, CanonicalNetworkId, FleetBinding, FleetId, FleetKey, ReleaseBuildNonce},
        storage::stable::fleet_activation::{
            FleetActivationEvidenceRecord, FleetActivationStateRecord,
            FleetCascadeActivationEvidenceRecord, FleetCredentialGenerationRefRecord,
            FleetCredentialManifestRecord,
        },
    };

    fn release_build(byte: u8) -> ReleaseBuildId {
        ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([byte; 32]))
    }

    fn input(release_build_id: ReleaseBuildId) -> CurrentRootInstallIdentity {
        CurrentRootInstallIdentity {
            fleet: FleetBinding {
                fleet: FleetKey {
                    network: CanonicalNetworkId::public_ic(),
                    fleet_id: FleetId::from_generated_bytes([11; 32]),
                },
                app: AppId::from("toko"),
            },
            install_id: [12; 32],
            release_build_id,
            expected_module_hash: Some([13; 32]),
        }
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
        let identity =
            FleetActivationOps::initialize_root_prepared(input(release_build_id), release_build_id)
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
            FleetActivationOps::initialize_root_prepared(
                input(release_build_id),
                release_build_id,
            )
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
            FleetActivationOps::initialize_root_prepared(input(supplied), embedded),
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
        let identity = FleetActivationOps::initialize_nonroot_prepared(
            root_input.fleet,
            root_input.install_id,
            root_input.release_build_id,
            release_build_id,
            Some(vec![33, 34]),
        )
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
            FleetActivationOps::initialize_nonroot_prepared(
                root_input.fleet,
                root_input.install_id,
                root_input.release_build_id,
                embedded,
                None,
            ),
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
    fn status_projects_the_exact_prepared_identity() {
        FleetActivationOps::reset_for_tests();
        let release_build_id = release_build(17);
        FleetActivationOps::initialize_root_prepared(input(release_build_id), release_build_id)
            .expect("initialize Prepared");

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
        FleetActivationOps::initialize_root_prepared(input(release_build_id), release_build_id)
            .expect("initialize Prepared");
        let mut data = FleetActivationOps::snapshot();
        data.record
            .as_mut()
            .expect("record")
            .credential_manifests
            .push(FleetCredentialManifestRecord {
                fleet: FleetKey {
                    network: CanonicalNetworkId::public_ic(),
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
        FleetActivationOps::initialize_root_prepared(input(release_build_id), release_build_id)
            .expect("initialize Prepared");
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
        FleetActivationOps::initialize_root_prepared(input(release_build_id), release_build_id)
            .expect("initialize Prepared");
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
        FleetActivationOps::initialize_root_prepared(input(release_build_id), release_build_id)
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
        FleetActivationOps::initialize_nonroot_prepared(
            root_input.fleet,
            root_input.install_id,
            root_input.release_build_id,
            release_build_id,
            Some(vec![35, 36]),
        )
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
        FleetActivationOps::initialize_nonroot_prepared(
            root_input.fleet,
            root_input.install_id,
            root_input.release_build_id,
            release_build_id,
            None,
        )
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
