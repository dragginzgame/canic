//! Module: ops::fleet_activation
//!
//! Responsibility: encode and hash exact Fleet-activation evidence objects.
//! Does not own: activation sequencing, endpoint authorization, or stable mutation.
//! Boundary: workflow supplies validated DTOs; this module returns domain-separated hashes.

use crate::{
    InternalError, InternalErrorOrigin,
    dto::{
        cascade::{
            StateSnapshotInput, TopologyChildren, TopologyDirectChild, TopologyPathNode,
            TopologySnapshotInput,
        },
        fleet_activation::{
            FleetActivationIdentity, FleetCascadeActivationEvidence, FleetCascadeManifestEntry,
            FleetCredentialGenerationRef, FleetCredentialManifest, FleetCredentialManifestEntry,
            MAX_FLEET_ACTIVATION_CANISTERS, MAX_FLEET_CREDENTIAL_MANIFEST_ENTRIES,
        },
        state::FleetStateInput,
        topology::{
            DirectoryEntryInput, DirectoryProvenance, FleetDirectoryInput, SubnetDirectoryInput,
        },
    },
    ids::{FleetBinding, FleetKey},
};
use ciborium::value::Value;
use sha2::{Digest, Sha256};

const STATE_SNAPSHOT_DOMAIN: &[u8] = b"canic:fleet-activation:state-snapshot\0";
const TOPOLOGY_SNAPSHOT_DOMAIN: &[u8] = b"canic:fleet-activation:topology-snapshot\0";
const CASCADE_MANIFEST_DOMAIN: &[u8] = b"canic:fleet-activation:cascade-manifest\0";
const CREDENTIAL_POLICY_SET_DOMAIN: &[u8] = b"canic:fleet-activation:credential-root-policy-set\0";
const CREDENTIAL_TEMPLATE_SET_DOMAIN: &[u8] =
    b"canic:fleet-activation:credential-renewal-template-set\0";
const CREDENTIAL_MANIFEST_DOMAIN: &[u8] = b"canic:fleet-activation:credential-manifest\0";
const ACTIVATION_EVIDENCE_DOMAIN: &[u8] = b"canic:fleet-activation:activation-evidence\0";

pub struct FleetActivationEvidenceOps;

impl FleetActivationEvidenceOps {
    pub fn state_snapshot_hash(value: &StateSnapshotInput) -> Result<[u8; 32], InternalError> {
        hash_value(STATE_SNAPSHOT_DOMAIN, encode_state_snapshot(value)?)
    }

    pub fn topology_snapshot_hash(
        value: &TopologySnapshotInput,
    ) -> Result<[u8; 32], InternalError> {
        hash_value(TOPOLOGY_SNAPSHOT_DOMAIN, encode_topology_snapshot(value)?)
    }

    pub fn cascade_manifest_hash(
        value: &[FleetCascadeManifestEntry],
    ) -> Result<[u8; 32], InternalError> {
        if value.len() >= MAX_FLEET_ACTIVATION_CANISTERS {
            return Err(canonical_error(
                "Fleet cascade manifest leaves no bounded inventory slot for the root Canister",
            ));
        }
        require_strict_bytes_order(
            value.iter().map(|entry| entry.principal.as_slice()),
            "Fleet cascade manifest",
        )?;
        hash_value(
            CASCADE_MANIFEST_DOMAIN,
            Value::Array(value.iter().map(encode_cascade_manifest_entry).collect()),
        )
    }

    pub fn credential_manifest_hash(
        value: &FleetCredentialManifest,
    ) -> Result<[u8; 32], InternalError> {
        if value.generation == 0 {
            return Err(canonical_error(
                "Fleet credential manifest generation must be greater than zero",
            ));
        }
        if value.entries.len() > MAX_FLEET_CREDENTIAL_MANIFEST_ENTRIES {
            return Err(canonical_error(
                "Fleet credential manifest exceeds its entry bound",
            ));
        }
        require_strict_bytes_order(
            value
                .entries
                .iter()
                .map(|entry| entry.subject_canister.as_slice()),
            "Fleet credential manifest",
        )?;
        hash_value(
            CREDENTIAL_MANIFEST_DOMAIN,
            encode_credential_manifest(value),
        )
    }

    pub fn empty_root_policy_set_hash() -> Result<[u8; 32], InternalError> {
        hash_value(CREDENTIAL_POLICY_SET_DOMAIN, Value::Array(Vec::new()))
    }

    pub fn empty_renewal_template_set_hash() -> Result<[u8; 32], InternalError> {
        hash_value(CREDENTIAL_TEMPLATE_SET_DOMAIN, Value::Array(Vec::new()))
    }

    pub fn activation_evidence_hash(
        identity: &FleetActivationIdentity,
        cascade: &FleetCascadeActivationEvidence,
        credential: FleetCredentialGenerationRef,
    ) -> Result<[u8; 32], InternalError> {
        if credential.generation == 0 {
            return Err(canonical_error(
                "Fleet activation credential generation must be greater than zero",
            ));
        }
        hash_value(
            ACTIVATION_EVIDENCE_DOMAIN,
            Value::Array(vec![
                encode_activation_identity(identity),
                encode_cascade_evidence(cascade),
                encode_credential_generation(credential),
            ]),
        )
    }
}

fn encode_state_snapshot(value: &StateSnapshotInput) -> Result<Value, InternalError> {
    Ok(Value::Array(vec![
        encode_optional(
            value
                .fleet_state
                .as_ref()
                .map(|fleet_state| encode_fleet_state(*fleet_state)),
        ),
        encode_optional(
            value
                .fleet_directory
                .as_ref()
                .map(encode_fleet_directory)
                .transpose()?,
        ),
        encode_optional(
            value
                .subnet_directory
                .as_ref()
                .map(encode_subnet_directory)
                .transpose()?,
        ),
    ]))
}

fn encode_fleet_state(value: FleetStateInput) -> Value {
    let mode = match value.mode {
        crate::domain::state::FleetMode::Enabled => 0,
        crate::domain::state::FleetMode::Readonly => 1,
        crate::domain::state::FleetMode::Disabled => 2,
    };
    Value::Array(vec![
        integer(mode),
        Value::Bool(value.cycles_funding_enabled),
    ])
}

fn encode_fleet_directory(value: &FleetDirectoryInput) -> Result<Value, InternalError> {
    encode_directory(
        &value.provenance,
        &value.entries,
        "Fleet Directory snapshot",
    )
}

fn encode_subnet_directory(value: &SubnetDirectoryInput) -> Result<Value, InternalError> {
    encode_directory(
        &value.provenance,
        &value.entries,
        "Subnet Directory snapshot",
    )
}

fn encode_directory(
    provenance: &DirectoryProvenance,
    entries: &[DirectoryEntryInput],
    label: &str,
) -> Result<Value, InternalError> {
    require_strict_bytes_order(
        entries.iter().map(|entry| entry.role.as_str().as_bytes()),
        label,
    )?;
    Ok(Value::Array(vec![
        encode_directory_provenance(provenance),
        Value::Array(entries.iter().map(encode_directory_entry).collect()),
    ]))
}

fn encode_directory_provenance(value: &DirectoryProvenance) -> Value {
    Value::Array(vec![
        encode_fleet_binding(&value.fleet),
        principal(value.source_root),
    ])
}

fn encode_directory_entry(value: &DirectoryEntryInput) -> Value {
    Value::Array(vec![
        byte_string(value.role.as_str().as_bytes()),
        principal(value.pid),
    ])
}

fn encode_topology_snapshot(value: &TopologySnapshotInput) -> Result<Value, InternalError> {
    require_strict_bytes_order(
        value
            .children_map
            .iter()
            .map(|entry| entry.parent_pid.as_slice()),
        "topology children map",
    )?;
    for entry in &value.children_map {
        require_strict_bytes_order(
            entry.children.iter().map(|child| child.pid.as_slice()),
            "topology child list",
        )?;
    }
    Ok(Value::Array(vec![
        Value::Array(
            value
                .parents
                .iter()
                .map(encode_topology_path_node)
                .collect(),
        ),
        Value::Array(
            value
                .children_map
                .iter()
                .map(encode_topology_children)
                .collect(),
        ),
    ]))
}

fn encode_topology_path_node(value: &TopologyPathNode) -> Value {
    Value::Array(vec![
        principal(value.pid),
        byte_string(value.role.as_str().as_bytes()),
        encode_optional(value.parent_pid.map(principal)),
    ])
}

fn encode_topology_children(value: &TopologyChildren) -> Value {
    Value::Array(vec![
        principal(value.parent_pid),
        Value::Array(
            value
                .children
                .iter()
                .map(encode_topology_direct_child)
                .collect(),
        ),
    ])
}

fn encode_topology_direct_child(value: &TopologyDirectChild) -> Value {
    Value::Array(vec![
        principal(value.pid),
        byte_string(value.role.as_str().as_bytes()),
    ])
}

fn encode_cascade_manifest_entry(value: &FleetCascadeManifestEntry) -> Value {
    Value::Array(vec![
        principal(value.principal),
        digest(value.state_snapshot_hash),
        digest(value.topology_snapshot_hash),
    ])
}

fn encode_credential_manifest(value: &FleetCredentialManifest) -> Value {
    Value::Array(vec![
        encode_fleet_key(&value.fleet),
        digest(value.activation_id),
        integer(value.generation),
        digest(value.root_policy_set_hash),
        digest(value.renewal_template_set_hash),
        Value::Array(
            value
                .entries
                .iter()
                .map(encode_credential_manifest_entry)
                .collect(),
        ),
    ])
}

fn encode_credential_manifest_entry(value: &FleetCredentialManifestEntry) -> Value {
    Value::Array(vec![
        principal(value.root_issuer),
        principal(value.subject_canister),
        integer(value.not_before_ns),
        integer(value.expires_at_ns),
        digest(value.key_identity_hash),
        digest(value.cert_hash),
        digest(value.proof_hash),
        digest(value.bundle_hash),
    ])
}

fn encode_activation_identity(value: &FleetActivationIdentity) -> Value {
    Value::Array(vec![
        encode_fleet_binding(&value.fleet),
        digest(value.operation_id),
        digest(*value.release_build_id.as_bytes()),
    ])
}

fn encode_cascade_evidence(value: &FleetCascadeActivationEvidence) -> Value {
    match value {
        FleetCascadeActivationEvidence::Source {
            cascade_manifest_hash,
        } => Value::Array(vec![integer(0), digest(*cascade_manifest_hash)]),
        FleetCascadeActivationEvidence::Applied {
            state_snapshot_hash,
            topology_snapshot_hash,
        } => Value::Array(vec![
            integer(1),
            digest(*state_snapshot_hash),
            digest(*topology_snapshot_hash),
        ]),
    }
}

fn encode_credential_generation(value: FleetCredentialGenerationRef) -> Value {
    Value::Array(vec![integer(value.generation), digest(value.manifest_hash)])
}

fn encode_fleet_binding(value: &FleetBinding) -> Value {
    Value::Array(vec![
        encode_fleet_key(&value.fleet),
        byte_string(value.app.as_str().as_bytes()),
    ])
}

fn encode_fleet_key(value: &FleetKey) -> Value {
    Value::Array(vec![
        digest(*value.canonical_network_id.as_bytes()),
        digest(*value.fleet_id.as_bytes()),
    ])
}

fn encode_optional(value: Option<Value>) -> Value {
    value.unwrap_or(Value::Null)
}

fn principal(value: candid::Principal) -> Value {
    byte_string(value.as_slice())
}

fn digest(value: [u8; 32]) -> Value {
    byte_string(&value)
}

fn byte_string(value: &[u8]) -> Value {
    Value::Bytes(value.to_vec())
}

fn integer(value: u64) -> Value {
    Value::Integer(value.into())
}

fn require_strict_bytes_order<'a>(
    values: impl IntoIterator<Item = &'a [u8]>,
    label: &str,
) -> Result<(), InternalError> {
    let mut previous: Option<&[u8]> = None;
    for value in values {
        if previous.is_some_and(|previous| previous >= value) {
            return Err(canonical_error(format!(
                "{label} entries are not in strict canonical order"
            )));
        }
        previous = Some(value);
    }
    Ok(())
}

fn hash_value(domain: &[u8], value: Value) -> Result<[u8; 32], InternalError> {
    let bytes = encode_value(&value);
    let length = u64::try_from(bytes.len())
        .map_err(|_| canonical_error("Fleet activation evidence length does not fit u64"))?;
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(length.to_be_bytes());
    hasher.update(bytes);
    Ok(hasher.finalize().into())
}

fn encode_value(value: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(value, &mut bytes)
        .expect("serializing fixed Fleet-activation CBOR values cannot fail");
    bytes
}

fn canonical_error(message: impl Into<String>) -> InternalError {
    InternalError::ops(InternalErrorOrigin::Ops, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{AppId, CanonicalNetworkId, FleetId, ReleaseBuildId, ReleaseBuildNonce};

    fn identity() -> FleetActivationIdentity {
        FleetActivationIdentity {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::public_ic(),
                    fleet_id: FleetId::from_generated_bytes([1; 32]),
                },
                app: AppId::from("app"),
            },
            operation_id: [2; 32],
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [3; 32],
            )),
        }
    }

    #[test]
    fn activation_evidence_hash_is_domain_separated_and_deterministic() {
        let identity = identity();
        let cascade = FleetCascadeActivationEvidence::Source {
            cascade_manifest_hash: [4; 32],
        };
        let credential = FleetCredentialGenerationRef {
            generation: 1,
            manifest_hash: [5; 32],
        };

        let first =
            FleetActivationEvidenceOps::activation_evidence_hash(&identity, &cascade, credential)
                .expect("hash evidence");
        let second =
            FleetActivationEvidenceOps::activation_evidence_hash(&identity, &cascade, credential)
                .expect("hash evidence");

        assert_eq!(first, second);
        assert_ne!(first, [0; 32]);
    }

    #[test]
    fn empty_state_snapshot_uses_the_exact_fixed_array_cbor() {
        let value = StateSnapshotInput {
            fleet_state: None,
            fleet_directory: None,
            subnet_directory: None,
        };

        assert_eq!(
            encode_value(&encode_state_snapshot(&value).expect("encode state snapshot")),
            [0x83, 0xf6, 0xf6, 0xf6]
        );
    }

    #[test]
    fn evidence_values_contain_no_forbidden_cbor_types() {
        let value = Value::Array(vec![
            encode_activation_identity(&identity()),
            encode_cascade_evidence(&FleetCascadeActivationEvidence::Applied {
                state_snapshot_hash: [6; 32],
                topology_snapshot_hash: [7; 32],
            }),
            encode_credential_generation(FleetCredentialGenerationRef {
                generation: 1,
                manifest_hash: [8; 32],
            }),
        ]);

        assert_only_canonical_value_kinds(&value);
    }

    #[test]
    fn evidence_sets_reject_noncanonical_order_and_duplicates() {
        let entry = |principal_byte| FleetCascadeManifestEntry {
            principal: candid::Principal::from_slice(&[principal_byte]),
            state_snapshot_hash: [9; 32],
            topology_snapshot_hash: [10; 32],
        };

        assert!(FleetActivationEvidenceOps::cascade_manifest_hash(&[entry(2), entry(1)]).is_err());
        assert!(FleetActivationEvidenceOps::cascade_manifest_hash(&[entry(1), entry(1)]).is_err());
    }

    #[test]
    fn activation_manifests_reject_entry_counts_over_the_frozen_bounds() {
        let cascade_entry = FleetCascadeManifestEntry {
            principal: candid::Principal::from_slice(&[1]),
            state_snapshot_hash: [9; 32],
            topology_snapshot_hash: [10; 32],
        };
        assert!(
            FleetActivationEvidenceOps::cascade_manifest_hash(&vec![
                cascade_entry;
                MAX_FLEET_ACTIVATION_CANISTERS
            ])
            .is_err()
        );

        let credential_entry = FleetCredentialManifestEntry {
            root_issuer: candid::Principal::from_slice(&[1]),
            subject_canister: candid::Principal::from_slice(&[2]),
            not_before_ns: 1,
            expires_at_ns: 2,
            key_identity_hash: [3; 32],
            cert_hash: [4; 32],
            proof_hash: [5; 32],
            bundle_hash: [6; 32],
        };
        let credential_manifest = FleetCredentialManifest {
            fleet: identity().fleet.fleet,
            activation_id: [7; 32],
            generation: 1,
            root_policy_set_hash: [8; 32],
            renewal_template_set_hash: [9; 32],
            entries: vec![credential_entry; MAX_FLEET_CREDENTIAL_MANIFEST_ENTRIES + 1],
        };
        assert!(
            FleetActivationEvidenceOps::credential_manifest_hash(&credential_manifest).is_err()
        );
    }

    fn assert_only_canonical_value_kinds(value: &Value) {
        match value {
            Value::Integer(_) | Value::Bytes(_) | Value::Bool(_) | Value::Null => {}
            Value::Array(values) => {
                for value in values {
                    assert_only_canonical_value_kinds(value);
                }
            }
            Value::Float(_) | Value::Text(_) | Value::Tag(_, _) | Value::Map(_) => {
                panic!("forbidden value in canonical activation evidence: {value:?}")
            }
            _ => panic!("unsupported value in canonical activation evidence: {value:?}"),
        }
    }
}
