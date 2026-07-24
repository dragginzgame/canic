//! Module: ops::storage::fleet_activation::mapper
//!
//! Responsibility: project the protected Fleet activation record into its public status DTO.
//! Does not own: storage access, mutation, activation policy, or endpoint authorization.
//! Boundary: contradictory protected records fail closed instead of producing a partial status.

use super::FleetActivationOpsError;
use crate::{
    dto::fleet_activation::{
        FleetActivationIdentity, FleetActivationPhase, FleetActivationStatusResponse,
        FleetCascadeActivationEvidence, FleetCascadeManifestEntry, FleetCredentialGenerationRef,
        FleetCredentialManifest, FleetCredentialManifestEntry,
    },
    storage::stable::fleet_activation::{
        FleetActivationEvidenceRecord, FleetActivationIdentityRecord, FleetActivationRecord,
        FleetActivationStateRecord, FleetCascadeActivationEvidenceRecord,
        FleetCascadeManifestEntryRecord, FleetCredentialGenerationRefRecord,
        FleetCredentialManifestEntryRecord, FleetCredentialManifestRecord,
        MAX_RETAINED_PREPARED_CREDENTIAL_GENERATIONS,
    },
};
use std::collections::BTreeSet;

pub(super) fn record_to_status(
    record: FleetActivationRecord,
    is_root: bool,
) -> Result<FleetActivationStatusResponse, FleetActivationOpsError> {
    let FleetActivationRecord {
        state,
        cascade_manifest,
        credential_manifests,
    } = record;
    let (phase, identity, evidence, activated_at_ns) = match state {
        FleetActivationStateRecord::Prepared { identity, evidence } => {
            (FleetActivationPhase::Prepared, identity, evidence, None)
        }
        FleetActivationStateRecord::Active {
            identity,
            evidence,
            activated_at_ns,
        } => {
            if evidence.cascade.is_none() || evidence.credential.is_none() {
                return Err(invalid(
                    "Active Fleet activation is missing cascade or credential evidence",
                ));
            }
            (
                FleetActivationPhase::Active,
                identity,
                evidence,
                Some(activated_at_ns),
            )
        }
    };
    let FleetActivationEvidenceRecord {
        cascade,
        credential,
    } = evidence;

    let cascade_manifest = if is_root {
        match (&cascade, cascade_manifest) {
            (None, None) => None,
            (Some(FleetCascadeActivationEvidenceRecord::Source { .. }), Some(cascade_manifest)) => {
                Some(
                    cascade_manifest
                        .into_iter()
                        .map(cascade_manifest_entry_record_to_dto)
                        .collect(),
                )
            }
            (Some(FleetCascadeActivationEvidenceRecord::Applied { .. }), _) => {
                return Err(invalid(
                    "root Fleet activation contains non-root applied cascade evidence",
                ));
            }
            _ => {
                return Err(invalid(
                    "root cascade evidence and source manifest do not agree",
                ));
            }
        }
    } else {
        if cascade_manifest.is_some() {
            return Err(invalid(
                "non-root Fleet activation retains a root-only cascade manifest",
            ));
        }
        if matches!(
            &cascade,
            Some(FleetCascadeActivationEvidenceRecord::Source { .. })
        ) {
            return Err(invalid(
                "non-root Fleet activation contains root source cascade evidence",
            ));
        }
        None
    };

    let credential_manifest = if is_root {
        current_root_credential_manifest(&identity, credential.as_ref(), credential_manifests)?
    } else {
        if !credential_manifests.is_empty() {
            return Err(invalid(
                "non-root Fleet activation retains root-only credential manifests",
            ));
        }
        None
    };

    Ok(FleetActivationStatusResponse {
        phase,
        identity: identity_record_to_dto(identity),
        cascade: cascade.map(cascade_record_to_dto),
        cascade_manifest,
        credential: credential.map(credential_record_to_dto),
        credential_manifest,
        activated_at_ns,
    })
}

fn current_root_credential_manifest(
    identity: &FleetActivationIdentityRecord,
    credential: Option<&FleetCredentialGenerationRefRecord>,
    manifests: Vec<FleetCredentialManifestRecord>,
) -> Result<Option<FleetCredentialManifest>, FleetActivationOpsError> {
    if manifests.len() > MAX_RETAINED_PREPARED_CREDENTIAL_GENERATIONS {
        return Err(invalid(
            "root Fleet activation retains more than two credential generations",
        ));
    }

    let mut generations = BTreeSet::new();
    for manifest in &manifests {
        if manifest.fleet != identity.fleet.fleet
            || manifest.activation_id != identity.operation_id
            || manifest.generation == 0
        {
            return Err(invalid(
                "root credential manifest is not bound to the protected Fleet activation",
            ));
        }
        if !generations.insert(manifest.generation) {
            return Err(invalid(
                "root Fleet activation retains duplicate credential generations",
            ));
        }
    }

    let Some(credential) = credential else {
        if manifests.is_empty() {
            return Ok(None);
        }
        return Err(invalid(
            "root credential manifests exist without a current generation reference",
        ));
    };
    if credential.generation == 0 {
        return Err(invalid(
            "root Fleet activation uses credential generation zero",
        ));
    }

    let mut matching = manifests
        .into_iter()
        .filter(|manifest| manifest.generation == credential.generation);
    let current = matching
        .next()
        .ok_or_else(|| invalid("current root credential manifest is missing"))?;
    if matching.next().is_some() {
        return Err(invalid("current root credential generation is not unique"));
    }
    Ok(Some(credential_manifest_record_to_dto(current)))
}

fn identity_record_to_dto(record: FleetActivationIdentityRecord) -> FleetActivationIdentity {
    FleetActivationIdentity {
        fleet: record.fleet,
        operation_id: record.operation_id,
        release_build_id: record.release_build_id,
    }
}

const fn cascade_record_to_dto(
    record: FleetCascadeActivationEvidenceRecord,
) -> FleetCascadeActivationEvidence {
    match record {
        FleetCascadeActivationEvidenceRecord::Source {
            cascade_manifest_hash,
        } => FleetCascadeActivationEvidence::Source {
            cascade_manifest_hash,
        },
        FleetCascadeActivationEvidenceRecord::Applied {
            state_snapshot_hash,
            topology_snapshot_hash,
        } => FleetCascadeActivationEvidence::Applied {
            state_snapshot_hash,
            topology_snapshot_hash,
        },
    }
}

const fn cascade_manifest_entry_record_to_dto(
    record: FleetCascadeManifestEntryRecord,
) -> FleetCascadeManifestEntry {
    FleetCascadeManifestEntry {
        principal: record.principal,
        state_snapshot_hash: record.state_snapshot_hash,
        topology_snapshot_hash: record.topology_snapshot_hash,
    }
}

const fn credential_record_to_dto(
    record: FleetCredentialGenerationRefRecord,
) -> FleetCredentialGenerationRef {
    FleetCredentialGenerationRef {
        generation: record.generation,
        manifest_hash: record.manifest_hash,
    }
}

fn credential_manifest_record_to_dto(
    record: FleetCredentialManifestRecord,
) -> FleetCredentialManifest {
    FleetCredentialManifest {
        fleet: record.fleet,
        activation_id: record.activation_id,
        generation: record.generation,
        root_policy_set_hash: record.root_policy_set_hash,
        renewal_template_set_hash: record.renewal_template_set_hash,
        entries: record
            .entries
            .into_iter()
            .map(credential_manifest_entry_record_to_dto)
            .collect(),
    }
}

const fn credential_manifest_entry_record_to_dto(
    record: FleetCredentialManifestEntryRecord,
) -> FleetCredentialManifestEntry {
    FleetCredentialManifestEntry {
        root_issuer: record.root_issuer,
        subject_canister: record.subject_canister,
        not_before_ns: record.not_before_ns,
        expires_at_ns: record.expires_at_ns,
        key_identity_hash: record.key_identity_hash,
        cert_hash: record.cert_hash,
        proof_hash: record.proof_hash,
        bundle_hash: record.bundle_hash,
    }
}

fn invalid(reason: impl Into<String>) -> FleetActivationOpsError {
    FleetActivationOpsError::InvalidRecord {
        reason: reason.into(),
    }
}
