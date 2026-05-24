use crate::pic_role_attestation_support::*;
use canic_core::dto::{
    auth::{
        CanicInternalCallEnvelopeV1, CanicInternalCallHeaderV1, InternalInvocationProofPayloadV1,
        SignedInternalInvocationProofV1,
    },
    placement::directory::DirectoryEntryStatusResponse,
};

#[test]
fn generated_project_hub_client_calls_protected_project_instance() {
    test_progress(
        "generated_project_hub_client_calls_protected_project_instance",
        "setup root+project_hub",
    );
    let setup = install_test_root_with_verifier_cached();
    let pic = setup.pic.pic();
    let root_id = setup.root_id;
    let project_hub_id = setup
        .verifier_id
        .expect("project_hub verifier fixture should be installed");

    test_progress(
        "generated_project_hub_client_calls_protected_project_instance",
        "resolve project instance",
    );
    let status: Result<DirectoryEntryStatusResponse, Error> = update_call_as(
        pic,
        project_hub_id,
        Principal::anonymous(),
        "resolve_project",
        ("alpha".to_string(),),
    );
    let instance_id = match status.expect("project resolve should succeed") {
        DirectoryEntryStatusResponse::Bound { instance_pid, .. } => instance_pid,
        other @ DirectoryEntryStatusResponse::Pending { .. } => {
            panic!("project resolve should bind an instance, got {other:?}")
        }
    };
    wait_until_ready(pic, instance_id, 240);

    test_progress(
        "generated_project_hub_client_calls_protected_project_instance",
        "protected generated client call",
    );
    let generated_client_call: Result<(), Error> = update_call_as(
        pic,
        project_hub_id,
        Principal::anonymous(),
        "notify_project_instance",
        (instance_id, "alpha".to_string()),
    );
    generated_client_call.expect("generated protected client call should succeed");

    test_progress(
        "generated_project_hub_client_calls_protected_project_instance",
        "raw protected target rejection",
    );
    let raw_call = pic.update_call_as::<Result<(), Error>, _>(
        instance_id,
        project_hub_id,
        "project_instance_record_visit",
        ("alpha".to_string(),),
    );
    let raw_error = raw_call
        .expect("protected endpoint should return a typed Canic error instead of trapping")
        .expect_err("raw calls to protected instance endpoint must be rejected");
    assert_eq!(raw_error.code, ErrorCode::InternalRpcMalformed);

    test_progress(
        "generated_project_hub_client_calls_protected_project_instance",
        "invalid typed envelope rejection",
    );
    let invalid_envelope_call = pic.update_call_as::<Result<(), Error>, _>(
        instance_id,
        project_hub_id,
        "project_instance_record_visit",
        (invalid_internal_envelope(instance_id, project_hub_id),),
    );
    let invalid_envelope_error = invalid_envelope_call
        .expect("invalid envelope should return a typed Canic error instead of trapping")
        .expect_err("invalid envelope must be rejected before proof verification");
    assert_eq!(invalid_envelope_error.code, ErrorCode::InternalRpcMalformed);

    let registered_instance = role_pid(pic, root_id, "project_instance", 120);
    assert_eq!(
        registered_instance, instance_id,
        "directory-created instance should be visible in root topology"
    );
}

fn invalid_internal_envelope(
    instance_id: Principal,
    project_hub_id: Principal,
) -> CanicInternalCallEnvelopeV1 {
    CanicInternalCallEnvelopeV1 {
        version: 2,
        header: CanicInternalCallHeaderV1 {
            target_canister: instance_id,
            target_method: "project_instance_record_visit".to_string(),
        },
        proof: SignedInternalInvocationProofV1 {
            payload: InternalInvocationProofPayloadV1 {
                subject: project_hub_id,
                role: CanisterRole::new("project_hub"),
                subnet_id: None,
                audience: instance_id,
                audience_method: "project_instance_record_visit".to_string(),
                issued_at: 0,
                expires_at: 1,
                epoch: 0,
            },
            signature: Vec::new(),
            key_id: 0,
        },
        args: Vec::new(),
    }
}
