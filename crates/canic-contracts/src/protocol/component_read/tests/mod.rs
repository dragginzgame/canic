//! Exact caller-selected operation identity and full child lifecycle metadata.
use super::*;
#[test]
fn child_allocation_read_preserves_operation_identity() {
    let operation = [7; 32];
    let bytes =
        ic_wire::native::encode_request(child_allocation::METHOD_ID, &operation, REQUEST_LIMITS)
            .unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<[u8; 32]>(
            child_allocation::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        )
        .unwrap(),
        operation
    );
    assert!(matches!(
        ic_wire::native::decode_request::<[u8; 32]>(
            crate::protocol::registry_read::status::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}
#[cfg(feature = "wire-schema")]
#[test]
fn child_allocation_schema_retains_lifecycle_and_failure_evidence() {
    ic_wire::typescript::native_schema::<
        Result<crate::dto::component_read::ChildAllocation, crate::dto::error::Error>,
    >()
    .unwrap();
}

#[test]
fn child_failure_and_creation_preserve_retry_and_uncertain_effect_evidence() {
    use crate::dto::{
        component_read::{ChildFailure, Creation},
        component_registry::{RootComponentChildAllocationFailure, RootComponentCreationEvidence},
    };
    let failure = RootComponentChildAllocationFailure {
        diagnostic_code: 7,
        failed_at_ns: u64::MAX - 1,
        consecutive_failures: u32::MAX,
        retry_at_ns: u64::MAX,
        platform_rejection: Some("bounded rejection evidence".into()),
    };
    let bytes = ic_wire::native::encode_response(
        child_allocation::METHOD_ID,
        &ChildFailure::from(failure.clone()),
        RESPONSE_LIMITS,
    )
    .unwrap();
    let restored = ic_wire::native::decode_response::<ChildFailure>(
        child_allocation::METHOD_ID,
        &bytes,
        RESPONSE_LIMITS,
    )
    .unwrap();
    assert_eq!(RootComponentChildAllocationFailure::from(restored), failure);
    let principal = ic_platform::principal::ExternalPrincipal::from_slice(&[3]);
    for canister in [None, Some(principal)] {
        let creation = RootComponentCreationEvidence {
            wasm_store: principal,
            payload_hash: [4; 32],
            payload_size_bytes: u64::MAX,
            initial_cycles: u128::MAX.into(),
            controller: principal,
            canister,
        };
        let bytes = ic_wire::native::encode_response(
            child_allocation::METHOD_ID,
            &Creation::from(creation.clone()),
            RESPONSE_LIMITS,
        )
        .unwrap();
        let restored = ic_wire::native::decode_response::<Creation>(
            child_allocation::METHOD_ID,
            &bytes,
            RESPONSE_LIMITS,
        )
        .unwrap();
        assert_eq!(RootComponentCreationEvidence::from(restored), creation);
    }
}

#[test]
fn provisioning_and_child_reads_have_distinct_operation_frames() {
    let operation = [9; 32];
    let bytes =
        ic_wire::native::encode_request(provisioning::METHOD_ID, &operation, REQUEST_LIMITS)
            .unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<[u8; 32]>(
            provisioning::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        )
        .unwrap(),
        operation
    );
    assert!(matches!(
        ic_wire::native::decode_request::<[u8; 32]>(
            child_allocation::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}
#[cfg(feature = "wire-schema")]
#[test]
fn provisioning_schema_retains_full_result_and_recovery_authority() {
    ic_wire::typescript::native_schema::<
        Result<crate::dto::component_read::Provisioning, crate::dto::error::Error>,
    >()
    .unwrap();
}

#[test]
fn coordinator_provisioning_identity_is_distinct_from_root_provisioning() {
    let operation = [9_u8; 32];
    let bytes = ic_wire::native::encode_request(
        coordinator_provisioning::METHOD_ID,
        &operation,
        REQUEST_LIMITS,
    )
    .unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<[u8; 32]>(
            coordinator_provisioning::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        )
        .unwrap(),
        operation
    );
    assert!(matches!(
        ic_wire::native::decode_request::<[u8; 32]>(
            provisioning::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn coordinator_provisioning_schema_retains_full_progress_and_failure_origin() {
    ic_wire::typescript::native_schema::<
        Result<crate::dto::component_read::CoordinatorProvisioning, crate::dto::error::Error>,
    >()
    .unwrap();
}

#[test]
fn coordinator_failure_keeps_original_owner_deadline() {
    use crate::dto::{
        component_provisioning::ProvisioningFailureOrigin, component_read::FailureOrigin,
    };
    use crate::values::provisioning_failure::{
        ProvisioningFailureStage, ProvisioningRetryCategory,
    };
    for retry_at_ns in [None, Some(u64::MAX)] {
        let native = FailureOrigin {
            dependency: None,
            failed_at_ns: 41,
            retry_at_ns,
            stage: ProvisioningFailureStage::ComponentMembership,
            target: ic_platform::Principal::from_slice(&[42]),
            operation_id: [43; 32],
            diagnostic_code: 137,
            retry_category: ProvisioningRetryCategory::Backoff,
        };
        let canonical = ProvisioningFailureOrigin::from(native.clone());
        assert_eq!(canonical.retry_at_ns, retry_at_ns);
        assert_eq!(canonical.failed_at_ns, 41);
        assert_eq!(FailureOrigin::from(canonical), native);
    }
}

#[test]
fn allocation_read_has_its_own_operation_frame() {
    let operation = [17_u8; 32];
    let bytes =
        ic_wire::native::encode_request(allocation::METHOD_ID, &operation, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<[u8; 32]>(allocation::METHOD_ID, &bytes, REQUEST_LIMITS)
            .unwrap(),
        operation
    );
    for other in [child_allocation::METHOD_ID, provisioning::METHOD_ID] {
        assert!(matches!(
            ic_wire::native::decode_request::<[u8; 32]>(other, &bytes, REQUEST_LIMITS),
            Err(ic_wire::Error::Identity)
        ));
    }
}

#[cfg(feature = "wire-schema")]
#[test]
fn allocation_schema_covers_complete_authority_and_installation_evidence() {
    ic_wire::typescript::native_schema::<
        Result<crate::dto::component_read::ComponentAllocation, crate::dto::error::Error>,
    >()
    .unwrap();
}

#[test]
fn membership_frames_preserve_subject_and_negative_result() {
    use crate::dto::component_registry::{RootMembershipRequest, RootMembershipResponse};
    let request = RootMembershipRequest {
        subject: ic_platform::Principal::from_slice(&[17]),
    };
    let bytes =
        ic_wire::native::encode_request(membership::METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<RootMembershipRequest>(
            membership::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        )
        .unwrap(),
        request
    );
    assert!(matches!(
        ic_wire::native::decode_request::<RootMembershipRequest>(
            allocation::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
    let response: Result<RootMembershipResponse, crate::dto::error::Error> =
        Ok(RootMembershipResponse { member: None });
    let bytes = ic_wire::native::encode_response(membership::METHOD_ID, &response, RESPONSE_LIMITS)
        .unwrap();
    let decoded = ic_wire::native::decode_response::<
        Result<RootMembershipResponse, crate::dto::error::Error>,
    >(membership::METHOD_ID, &bytes, RESPONSE_LIMITS)
    .unwrap();
    assert_eq!(decoded, response);
}

#[cfg(feature = "wire-schema")]
#[test]
fn membership_schema_covers_native_subject_and_both_managed_binding_kinds() {
    ic_wire::typescript::native_schema::<crate::dto::component_registry::RootMembershipRequest>()
        .unwrap();
    ic_wire::typescript::native_schema::<
        Result<crate::dto::component_registry::RootMembershipResponse, crate::dto::error::Error>,
    >()
    .unwrap();
}
