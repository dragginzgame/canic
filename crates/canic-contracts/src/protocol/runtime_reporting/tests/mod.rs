//! Full retained evidence survives native frames without changing canonical stored identities.
use super::*;
use crate::{
    dto::{fleet_activation as activation, runtime_reporting as native},
    ids::{AppId, CanonicalNetworkId, FleetBinding, FleetId, FleetKey, ReleaseBuildId},
};
use ic_platform::principal::ExternalPrincipal;

fn fleet() -> FleetBinding {
    FleetBinding {
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([1; 32]),
        },
        app: AppId::from("fixture"),
    }
}

#[test]
fn maximum_activation_manifest_preserves_original_credentials_and_hashes() {
    let principal = ExternalPrincipal::from_slice(&[0xff; 29]);
    let original = activation::FleetActivationStatusResponse {
        phase: activation::FleetActivationPhase::Active,
        identity: activation::FleetActivationIdentity {
            fleet: fleet(),
            operation_id: [2; 32],
            release_build_id: ReleaseBuildId::from_nonce(
                crate::ids::ReleaseBuildNonce::from_random_bytes([3; 32]),
            ),
        },
        cascade: Some(activation::FleetCascadeActivationEvidence::Applied {
            state_snapshot_hash: [4; 32],
            topology_snapshot_hash: [5; 32],
        }),
        cascade_manifest: Some(vec![
            activation::FleetCascadeManifestEntry {
                principal,
                state_snapshot_hash: [6; 32],
                topology_snapshot_hash: [7; 32]
            };
            activation::MAX_FLEET_ACTIVATION_CANISTERS
        ]),
        credential: Some(activation::FleetCredentialGenerationRef {
            generation: u64::MAX,
            manifest_hash: [8; 32],
        }),
        credential_manifest: Some(activation::FleetCredentialManifest {
            fleet: fleet().fleet,
            activation_id: [9; 32],
            generation: u64::MAX,
            root_policy_set_hash: [10; 32],
            renewal_template_set_hash: [11; 32],
            entries: vec![
                activation::FleetCredentialManifestEntry {
                    root_issuer: principal,
                    subject_canister: principal,
                    not_before_ns: 1,
                    expires_at_ns: u64::MAX,
                    key_identity_hash: [12; 32],
                    cert_hash: [13; 32],
                    proof_hash: [14; 32],
                    bundle_hash: [15; 32]
                };
                activation::MAX_FLEET_CREDENTIAL_MANIFEST_ENTRIES
            ],
        }),
        activated_at_ns: Some(u64::MAX),
    };
    let canonical = serde_json::to_vec(&original).unwrap();
    let frame = ic_wire::native::encode_response(
        METHOD_ID,
        &Ok::<_, crate::dto::error::Error>(native::FleetActivationStatusResponse::from(
            original.clone(),
        )),
        RESPONSE_LIMITS,
    )
    .unwrap();
    let decoded: Result<native::FleetActivationStatusResponse, crate::dto::error::Error> =
        ic_wire::native::decode_response(METHOD_ID, &frame, RESPONSE_LIMITS).unwrap();
    let restored = activation::FleetActivationStatusResponse::from(decoded.unwrap());
    assert_eq!(restored, original);
    assert_eq!(serde_json::to_vec(&restored).unwrap(), canonical);
}

#[test]
fn operation_request_rejects_wrong_method_and_trailing_data() {
    let request = crate::dto::role::OperationStatusRequest {
        operation_id: [u8::MAX; 32],
    };
    for byte in [0, 127, 128, u8::MAX] {
        let input = crate::dto::role::OperationStatusRequest {
            operation_id: [byte; 32],
        };
        let frame = ic_wire::native::encode_request(METHOD_ID, &input, REQUEST_LIMITS).unwrap();
        let decoded: crate::dto::role::OperationStatusRequest =
            ic_wire::native::decode_request(METHOD_ID, &frame, REQUEST_LIMITS).unwrap();
        assert_eq!(decoded, input);
    }
    let frame = ic_wire::native::encode_request(METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    let decoded: crate::dto::role::OperationStatusRequest =
        ic_wire::native::decode_request(METHOD_ID, &frame, REQUEST_LIMITS).unwrap();
    assert_eq!(decoded.operation_id, request.operation_id);
    assert!(matches!(
        ic_wire::native::decode_request::<crate::dto::role::OperationStatusRequest>(
            crate::protocol::runtime_configuration::METHOD_ID,
            &frame,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
    let mut trailing = frame;
    trailing.push(0);
    assert!(matches!(
        ic_wire::native::decode_request::<crate::dto::role::OperationStatusRequest>(
            METHOD_ID,
            &trailing,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Trailing)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn runtime_status_metadata_covers_fixture_deployment_and_activation_graphs() {
    ic_wire::typescript::native_schema::<crate::dto::role::OperationStatusRequest>().unwrap();
    ic_wire::typescript::native_schema::<
        Result<native::ComponentRuntimeOperationStatus, crate::dto::error::Error>,
    >()
    .unwrap();
}
