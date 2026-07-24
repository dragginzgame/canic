//! Module: ops::storage::fleet_activation
//!
//! Responsibility: validate, initialize, and project the protected Fleet activation record.
//! Does not own: lifecycle orchestration, embedded build lookup, endpoint policy, or timers.
//! Boundary: initialization writes `Prepared` once; status rejects invalid role/state projections.

mod mapper;

use crate::{
    dto::fleet_activation::{
        CurrentRootInstallIdentity, FleetActivationIdentity, FleetActivationStatusResponse,
    },
    ids::{FleetBinding, ReleaseBuildId},
    model::fleet_activation::{
        NonrootInstallIdentity, PrepareFleetActivationError, PreparedFleetActivation,
        RootInstallIdentity, prepare_nonroot_install, prepare_root_install,
    },
    storage::stable::fleet_activation::{
        FleetActivation, FleetActivationData, FleetActivationEvidenceRecord,
        FleetActivationIdentityRecord, FleetActivationRecord, FleetActivationStateRecord,
        MAX_FLEET_ACTIVATION_RECORD_BYTES,
    },
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
}

///
/// FleetActivationOps
///

pub struct FleetActivationOps;

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
        initialize_prepared(prepared)
    }

    pub(crate) fn initialize_nonroot_prepared(
        fleet: FleetBinding,
        install_id: [u8; 32],
        release_build_id: ReleaseBuildId,
        embedded_release_build_id: ReleaseBuildId,
    ) -> Result<FleetActivationIdentity, FleetActivationOpsError> {
        let prepared = prepare_nonroot_install(
            NonrootInstallIdentity {
                fleet,
                install_id,
                release_build_id,
            },
            embedded_release_build_id,
        )?;
        initialize_prepared(prepared)
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

    pub(crate) fn require_active(is_root: bool) -> Result<(), FleetActivationOpsError> {
        let status = Self::status(is_root)?;
        if status.phase != crate::dto::fleet_activation::FleetActivationPhase::Active {
            return Err(FleetActivationOpsError::NotActive);
        }
        Ok(())
    }

    #[cfg(test)]
    fn reset_for_tests() {
        FleetActivation::import(FleetActivationData::default());
    }
}

fn initialize_prepared(
    prepared: PreparedFleetActivation,
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
        },
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
    fn nonroot_init_commits_the_exact_empty_prepared_identity_once() {
        FleetActivationOps::reset_for_tests();
        let release_build_id = release_build(32);
        let root_input = input(release_build_id);
        let identity = FleetActivationOps::initialize_nonroot_prepared(
            root_input.fleet,
            root_input.install_id,
            root_input.release_build_id,
            release_build_id,
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
                ..
            }
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
}
