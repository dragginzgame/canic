//! Module: ops::storage::fleet_activation
//!
//! Responsibility: validate, convert, and initialize the protected Fleet activation record.
//! Does not own: lifecycle orchestration, embedded build lookup, endpoint policy, or timers.
//! Boundary: one successful initialization writes `Prepared`; an existing record fails closed.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the protected owner is staged before root lifecycle mutation is admitted"
    )
)]

use crate::{
    dto::fleet_activation::{CurrentRootInstallIdentity, FleetActivationIdentity},
    ids::ReleaseBuildId,
    model::fleet_activation::{
        PrepareFleetActivationError, RootInstallIdentity, prepare_root_install,
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
                expected_module_hash: input.expected_module_hash,
            },
            embedded_release_build_id,
        )?;
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

    #[must_use]
    pub(crate) fn snapshot() -> FleetActivationData {
        FleetActivation::export()
    }

    #[cfg(test)]
    fn reset_for_tests() {
        FleetActivation::import(FleetActivationData::default());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ids::{AppId, CanonicalNetworkId, FleetBinding, FleetId, FleetKey, ReleaseBuildNonce},
        storage::stable::fleet_activation::FleetActivationStateRecord,
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
}
