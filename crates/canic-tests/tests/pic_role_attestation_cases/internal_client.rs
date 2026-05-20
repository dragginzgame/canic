use crate::pic_role_attestation_support::*;
use canic_core::dto::placement::directory::DirectoryEntryStatusResponse;

#[test]
fn generated_project_hub_client_calls_protected_project_instance() {
    test_progress(
        "generated_project_hub_client_calls_protected_project_instance",
        "setup root+project_hub",
    );
    let setup = install_test_root_with_verifier_cached();
    let pic = PicBorrow(setup.pic.pic());
    let root_id = setup.root_id;
    let project_hub_id = setup
        .verifier_id
        .expect("project_hub verifier fixture should be installed");

    test_progress(
        "generated_project_hub_client_calls_protected_project_instance",
        "resolve project instance",
    );
    let status: Result<DirectoryEntryStatusResponse, Error> = update_call_as(
        &pic,
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
    wait_until_ready(&pic, instance_id, 240);

    test_progress(
        "generated_project_hub_client_calls_protected_project_instance",
        "protected generated client call",
    );
    let generated_client_call: Result<(), Error> = update_call_as(
        &pic,
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
    assert!(
        raw_call.is_err() || raw_call.is_ok_and(|response| response.is_err()),
        "raw calls to protected instance endpoint must be rejected"
    );

    let registered_instance = role_pid(&pic, root_id, "project_instance", 120);
    assert_eq!(
        registered_instance, instance_id,
        "directory-created instance should be visible in root topology"
    );
}
