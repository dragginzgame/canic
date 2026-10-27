//! Admission framing preserves reviewed request identity and original operation evidence.
use super::*;
use crate::{
    dto::{admission_control as native, error::Error, fleet_admission as stored},
    ids::*,
};
use ic_platform::principal::ExternalPrincipal;

fn request() -> stored::FleetAdmissionMutationRequest {
    let principal = ExternalPrincipal::from_slice(&[7; 29]);
    stored::FleetAdmissionMutationRequest {
        authority: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([3; 32]),
                },
                app: AppId::from("fixture"),
            },
            coordinator_subnet: SubnetId::from_principal(principal),
            coordinator: principal,
            recovery_controllers: vec![principal],
        },
        expected_generation: u64::MAX,
        expected_policy_digest: [1; 32],
        action: stored::FleetAdmissionMutationAction::Remove,
        selector: FleetAdmissionSelector::FleetSubnetRoot(SubnetId::from_principal(principal)),
        principal,
        operation_id: [2; 32],
        successor_policy_digest: [3; 32],
        participant_catalog_digest: [4; 32],
        participant_count: u32::MAX,
    }
}

#[test]
fn reviewed_mutation_is_lossless_and_cannot_decode_as_status() {
    let original = request();
    let canonical = serde_json::to_vec(&original).unwrap();
    let frame = ic_wire::native::encode_request(
        mutate::METHOD_ID,
        &native::MutationRequest::from(original.clone()),
        REQUEST_LIMITS,
    )
    .unwrap();
    let decoded: native::MutationRequest =
        ic_wire::native::decode_request(mutate::METHOD_ID, &frame, REQUEST_LIMITS).unwrap();
    let restored: stored::FleetAdmissionMutationRequest = decoded.into();
    assert_eq!(restored, original);
    assert_eq!(serde_json::to_vec(&restored).unwrap(), canonical);
    assert!(matches!(
        ic_wire::native::decode_request::<native::StatusRequest>(
            status::METHOD_ID,
            &frame,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[test]
fn operation_status_preserves_original_receipt_and_caller_evidence() {
    let original = request();
    let operation = stored::FleetAdmissionOperationStatusResponse {
        operation_id: original.operation_id,
        action: original.action,
        selector: original.selector,
        principal: original.principal,
        phase: stored::FleetAdmissionOperationPhase::Completed(
            stored::FleetAdmissionMutationResponse {
                outcome: stored::FleetAdmissionMutationOutcome::CatalogChanged,
                operation_id: original.operation_id,
                generation: original.expected_generation,
                policy_digest: original.expected_policy_digest,
            },
        ),
    };
    let reply: Result<native::OperationStatus, Error> = Ok(operation.clone().into());
    let frame =
        ic_wire::native::encode_response(status::METHOD_ID, &reply, RESPONSE_LIMITS).unwrap();
    let decoded: Result<native::OperationStatus, Error> =
        ic_wire::native::decode_response(status::METHOD_ID, &frame, RESPONSE_LIMITS).unwrap();
    assert_eq!(
        stored::FleetAdmissionOperationStatusResponse::from(decoded.unwrap()),
        operation
    );
}

#[cfg(feature = "wire-schema")]
#[test]
fn complete_admission_contract_has_native_schema_metadata() {
    ic_wire::typescript::native_schema::<native::MutationRequest>().unwrap();
    ic_wire::typescript::native_schema::<native::StatusRequest>().unwrap();
    ic_wire::typescript::native_schema::<Result<native::StatusResponse, Error>>().unwrap();
    ic_wire::typescript::native_schema::<Result<stored::FleetAdmissionMutationResponse, Error>>()
        .unwrap();
}

#[test]
fn root_admission_pages_preserve_current_and_retained_progress() {
    use crate::dto::page::Page;
    for phase in [
        None,
        Some(stored::FleetAdmissionRootTransitionPhase::Released),
        Some(stored::FleetAdmissionRootTransitionPhase::Converged),
    ] {
        let original = native::RootStatus {
            operation_id: Some([1; 32]),
            phase,
            active_generation: u64::MAX,
            active_policy_digest: [2; 32],
            successor_generation: Some(u64::MAX),
            successor_policy_digest: Some([3; 32]),
            participant_catalog_digest: Some([4; 32]),
            participants: Page {
                entries: vec![],
                total: u64::MAX,
            },
            maximum_page_size: u16::MAX,
            last_result: None,
        };
        let original_bytes = serde_json::to_vec(&original).unwrap();
        let stored: stored::FleetAdmissionRootStatusResponse = original.into();
        let projected: native::RootStatus = stored.into();
        assert_eq!(serde_json::to_vec(&projected).unwrap(), original_bytes);
        let frame = ic_wire::native::encode_response(
            root_status::METHOD_ID,
            &Ok::<_, Error>(projected),
            RESPONSE_LIMITS,
        )
        .unwrap();
        let decoded: Result<native::RootStatus, Error> =
            ic_wire::native::decode_response(root_status::METHOD_ID, &frame, RESPONSE_LIMITS)
                .unwrap();
        assert_eq!(
            serde_json::to_vec(&decoded.unwrap()).unwrap(),
            original_bytes
        );
        assert!(matches!(
            ic_wire::native::decode_response::<Result<native::RootStatus, Error>>(
                status::METHOD_ID,
                &frame,
                RESPONSE_LIMITS
            ),
            Err(ic_wire::Error::Identity)
        ));
    }
}

#[cfg(feature = "wire-schema")]
#[test]
fn root_admission_schema_includes_complete_receipt_and_participant_graph() {
    ic_wire::typescript::native_schema::<Result<native::RootStatus, Error>>().unwrap();
    ic_wire::typescript::native_schema::<native::RootReceipt>().unwrap();
}

#[cfg(feature = "wire-schema")]
#[test]
fn managed_admission_schema_contains_complete_native_projection() {
    ic_wire::typescript::native_schema::<Result<native::ManagedStatus, Error>>().unwrap();
}

#[test]
fn managed_admission_page_frames_are_distinct_from_root_pages() {
    use crate::dto::page::PageRequest;
    let page = PageRequest {
        limit: u64::MAX,
        offset: u64::MAX,
    };
    let bytes =
        ic_wire::native::encode_request(managed_status::METHOD_ID, &page, REQUEST_LIMITS).unwrap();
    let decoded: PageRequest =
        ic_wire::native::decode_request(managed_status::METHOD_ID, &bytes, REQUEST_LIMITS).unwrap();
    assert_eq!(decoded, page);
    assert!(matches!(
        ic_wire::native::decode_request::<PageRequest>(
            root_status::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
    let mut trailing = bytes;
    trailing.push(0);
    assert!(matches!(
        ic_wire::native::decode_request::<PageRequest>(
            managed_status::METHOD_ID,
            &trailing,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Trailing)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn target_transitions_have_complete_native_metadata() {
    ic_wire::typescript::native_schema::<native::PrepareTargetRequest>().unwrap();
    ic_wire::typescript::native_schema::<stored::FleetAdmissionActivateTargetRequest>().unwrap();
    ic_wire::typescript::native_schema::<stored::FleetAdmissionOpenTargetRequest>().unwrap();
    ic_wire::typescript::native_schema::<Result<native::TargetReceipt, Error>>().unwrap();
}

#[test]
fn target_mutation_identity_does_not_admit_another_transition() {
    let request = stored::FleetAdmissionOpenTargetRequest {
        operation_id: [1; 32],
        generation: u64::MAX,
        policy_digest: [2; 32],
        projection_digest: [3; 32],
    };
    let bytes =
        ic_wire::native::encode_request(target_open::METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    let decoded: stored::FleetAdmissionOpenTargetRequest =
        ic_wire::native::decode_request(target_open::METHOD_ID, &bytes, REQUEST_LIMITS).unwrap();
    assert_eq!(decoded, request);
    assert!(matches!(
        ic_wire::native::decode_request::<stored::FleetAdmissionOpenTargetRequest>(
            target_activate::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[test]
fn root_prepare_preserves_every_reservation_stage_and_authority() {
    for stage in [
        stored::FleetAdmissionPrepareRootStage::Reserve,
        stored::FleetAdmissionPrepareRootStage::Fence,
        stored::FleetAdmissionPrepareRootStage::Release,
    ] {
        let mutation = request();
        let original = stored::FleetAdmissionPrepareRootRequest {
            successor: FleetAdmissionPolicy {
                schema_version: 1,
                fleet: mutation.authority.fleet.clone(),
                generation: u64::MAX,
                fleet_principals: vec![mutation.principal],
                rules: vec![],
                policy_digest: [9; 32],
            },
            authority: mutation.authority,
            operation_id: mutation.operation_id,
            expected_generation: mutation.expected_generation,
            expected_policy_digest: mutation.expected_policy_digest,
            stage,
        };
        let bytes = ic_wire::native::encode_request(
            root_prepare::METHOD_ID,
            &native::PrepareRootRequest::from(original.clone()),
            REQUEST_LIMITS,
        )
        .unwrap();
        let decoded: native::PrepareRootRequest =
            ic_wire::native::decode_request(root_prepare::METHOD_ID, &bytes, REQUEST_LIMITS)
                .unwrap();
        assert_eq!(
            stored::FleetAdmissionPrepareRootRequest::from(decoded),
            original
        );
        assert!(matches!(
            ic_wire::native::decode_request::<native::PrepareRootRequest>(
                root_activate::METHOD_ID,
                &bytes,
                REQUEST_LIMITS,
            ),
            Err(ic_wire::Error::Identity)
        ));
        let mut trailing = bytes;
        trailing.push(0);
        assert!(matches!(
            ic_wire::native::decode_request::<native::PrepareRootRequest>(
                root_prepare::METHOD_ID,
                &trailing,
                REQUEST_LIMITS,
            ),
            Err(ic_wire::Error::Trailing)
        ));
    }
}

#[cfg(feature = "wire-schema")]
#[test]
fn root_transitions_have_complete_native_metadata() {
    ic_wire::typescript::native_schema::<native::PrepareRootRequest>().unwrap();
    ic_wire::typescript::native_schema::<native::ActivateRootRequest>().unwrap();
    ic_wire::typescript::native_schema::<native::OpenRootRequest>().unwrap();
    ic_wire::typescript::native_schema::<Result<native::RootReceipt, Error>>().unwrap();
}

#[test]
fn root_activation_and_open_preserve_exact_successor_identity() {
    let mutation = request();
    let activation = stored::FleetAdmissionActivateRootRequest {
        authority: mutation.authority.clone(),
        operation_id: mutation.operation_id,
        expected_generation: mutation.expected_generation,
        expected_policy_digest: mutation.expected_policy_digest,
        successor_generation: u64::MAX,
        successor_policy_digest: [9; 32],
    };
    let frame = ic_wire::native::encode_request(
        root_activate::METHOD_ID,
        &native::ActivateRootRequest::from(activation.clone()),
        REQUEST_LIMITS,
    )
    .unwrap();
    let decoded: native::ActivateRootRequest =
        ic_wire::native::decode_request(root_activate::METHOD_ID, &frame, REQUEST_LIMITS).unwrap();
    assert_eq!(
        stored::FleetAdmissionActivateRootRequest::from(decoded),
        activation
    );
    let open = stored::FleetAdmissionOpenRootRequest {
        authority: mutation.authority,
        operation_id: mutation.operation_id,
        generation: activation.successor_generation,
        policy_digest: activation.successor_policy_digest,
    };
    let frame = ic_wire::native::encode_request(
        root_open::METHOD_ID,
        &native::OpenRootRequest::from(open.clone()),
        REQUEST_LIMITS,
    )
    .unwrap();
    let decoded: native::OpenRootRequest =
        ic_wire::native::decode_request(root_open::METHOD_ID, &frame, REQUEST_LIMITS).unwrap();
    assert_eq!(stored::FleetAdmissionOpenRootRequest::from(decoded), open);
}

#[test]
fn operation_read_preserves_exact_id_and_rejects_another_method() {
    let operation_id = [0x42_u8; 32];
    let bytes = ic_wire::native::encode_request(
        super::operation::METHOD_ID,
        &operation_id,
        super::REQUEST_LIMITS,
    )
    .unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<[u8; 32]>(
            super::operation::METHOD_ID,
            &bytes,
            super::REQUEST_LIMITS
        )
        .unwrap(),
        operation_id
    );
    assert!(matches!(
        ic_wire::native::decode_request::<[u8; 32]>(
            crate::protocol::public_overview::METHOD_ID,
            &bytes,
            super::REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn operation_read_registers_complete_native_evidence() {
    ic_wire::typescript::native_schema::<
        Result<crate::dto::admission_control::OperationStatus, crate::dto::error::Error>,
    >()
    .unwrap();
}
