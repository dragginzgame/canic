use super::*;
use crate::{
    dto::component_deployment::ProtectedComponentDeployment,
    ids::{ComponentChildBinding, ManagedCanisterBinding, ReleaseBuildId, ReleaseBuildNonce},
    ops::storage::fleet_activation::protected_component_deployment_dto_to_record,
    storage::stable::fleet_activation::{
        ComponentRuntimeRecord, FleetActivationEvidenceRecord, FleetActivationIdentityRecord,
    },
};
use sha2::{Digest, Sha256};

fn fixture_record(child: bool) -> FleetActivationRecord {
    let ManagedCanisterBinding::Component(component) =
        crate::test::support::managed_component_binding()
    else {
        unreachable!()
    };
    let target = if child {
        ManagedCanisterBinding::ComponentChild(ComponentChildBinding {
            parent_canister_id: component.canister_id,
            canister_id: Principal::from_slice(&[15; 29]),
            role: "child".into(),
            component: component.clone(),
        })
    } else {
        ManagedCanisterBinding::Component(component.clone())
    };
    let descriptor = FixtureDescriptor {
        schema_version: 1,
        format_hash: [1; 32],
        encoded_length: 3,
        chunks: vec![FixtureChunkDescriptor {
            digest: Sha256::digest(b"row").into(),
            length: 3,
        }],
        completion_summary: [2; 32],
    };
    let identity = FleetActivationIdentityRecord {
        fleet: component.authority.binding.fleet.clone(),
        operation_id: [3; 32],
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([4; 32])),
    };
    let assignment = FixtureAssignment {
        store: Principal::from_slice(&[16; 29]),
        grant: FixtureGrant {
            revision: 1,
            enabled: true,
            binding: FixtureTargetBinding {
                target: target.clone(),
                installation: identity.operation_id,
                release_build_id: identity.release_build_id,
                content_id: fixture_content::content_id(&descriptor).unwrap(),
            },
        },
        descriptor,
    };
    FleetActivationRecord {
        state: FleetActivationStateRecord::Prepared {
            identity,
            evidence: FleetActivationEvidenceRecord {
                cascade: None,
                credential: None,
            },
            application_init_args: None,
        },
        root_authority: None,
        wasm_store_authority: None,
        prepared_state_snapshot_hash: None,
        prepared_topology_snapshot_hash: None,
        cascade_manifest: None,
        credential_manifests: Vec::new(),
        component_runtime: Some(ComponentRuntimeRecord {
            fixture: Some(to_record(assignment)),
            binding: target,
            deployment: protected_component_deployment_dto_to_record(
                ProtectedComponentDeployment::UngroupedOrdinary { binding: component },
            ),
            directory: None,
            activation: None,
        }),
    }
}

#[test]
fn fixture_assignment_roundtrips_for_parent_and_child_without_a_progress_cursor() {
    for child in [false, true] {
        let mut record = fixture_record(child);
        record
            .component_runtime
            .as_mut()
            .unwrap()
            .fixture
            .as_mut()
            .unwrap()
            .grant_revision = 3;
        validate(&record).unwrap();
        let bytes = crate::cdk::serialize::serialize(&record).unwrap();
        let restored: FleetActivationRecord = crate::cdk::serialize::deserialize(&bytes).unwrap();
        assert_eq!(restored, record);
        validate(&restored).unwrap();
        let status =
            crate::ops::storage::fleet_activation::component_runtime_status(restored).unwrap();
        let assignment = status.fixture.unwrap();
        assert_eq!(assignment.grant.binding.target, status.binding);
        assert_eq!(assignment.grant.binding.installation, status.operation_id);
        fixture_content::verify_chunk(&assignment.descriptor, 0, b"row").unwrap();
    }
}

#[test]
fn fixture_assignment_rejects_substitution_and_revocation_after_restore() {
    for child in [false, true] {
        let original = fixture_record(child);
        let changes: &[fn(&mut FixtureAssignmentRecord)] = &[
            |f| f.installation = [8; 32],
            |f| {
                f.release_build_id =
                    ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([8; 32]));
            },
            |f| f.content_id = [8; 32],
            |f| f.format_hash = [8; 32],
            |f| f.completion_summary = [8; 32],
            |f| f.chunks[0].digest = [8; 32],
            |f| f.encoded_length += 1,
            |f| f.schema_version = 2,
            |f| f.grant_revision = 0,
            |f| f.grant_enabled = false,
            |f| f.store = Principal::anonymous(),
            |f| f.store = Principal::management_canister(),
            |f| f.target = crate::test::support::managed_component_binding(),
        ];
        for (index, change) in changes.iter().enumerate() {
            // The final substitution changes a child into its parent.
            if index == changes.len() - 1 && !child {
                continue;
            }
            let mut record = original.clone();
            change(
                record
                    .component_runtime
                    .as_mut()
                    .unwrap()
                    .fixture
                    .as_mut()
                    .unwrap(),
            );
            assert!(matches!(
                validate(&record),
                Err(FleetActivationOpsError::InvalidRecord { .. })
            ));
            assert!(matches!(
                crate::ops::storage::fleet_activation::component_runtime_status(record),
                Err(FleetActivationOpsError::InvalidRecord { .. })
            ));
        }
    }
}

#[test]
fn fixture_assignment_rejects_same_principal_from_another_component_authority() {
    let mut record = fixture_record(false);
    let runtime = record.component_runtime.as_mut().unwrap();
    let ManagedCanisterBinding::Component(binding) = &mut runtime.fixture.as_mut().unwrap().target
    else {
        unreachable!()
    };
    binding.authority.epoch += 1;
    assert!(matches!(
        validate(&record),
        Err(FleetActivationOpsError::InvalidRecord { .. })
    ));
}
