use super::*;

#[test]
fn verify_registered_deployment_root_promotes_unverified_state() {
    let (root, check) = demo_unverified_registered_root_check("canic-root-verify-promote");

    let receipt = verify_registered_deployment_root(VerifyDeploymentRootOptions {
        deployment_name: "demo-local".to_string(),
        network: "local".to_string(),
        deployment_check: check,
        verified_at_unix_secs: Some(100),
        icp_root: Some(root.clone()),
    })
    .expect("verify registered root");
    let state = read_deployment_install_state(&root, "local", "demo-local")
        .expect("read verified state")
        .expect("state exists");

    assert_eq!(state.root_verification, RootVerificationStatus::Verified);
    assert_eq!(state.updated_at_unix_secs, 100);
    assert_eq!(
        receipt.state_transition,
        crate::deployment_truth::DeploymentRootVerificationStateTransitionV1::PromotedNotVerifiedToVerified
    );
    assert_eq!(
        receipt.previous_root_verification,
        crate::deployment_truth::DeploymentRootVerificationStateV1::NotVerified
    );
    assert_eq!(
        receipt.new_root_verification,
        crate::deployment_truth::DeploymentRootVerificationStateV1::Verified
    );
    assert_eq!(receipt.source_check_id, "local:local:demo-local:check");
    assert_eq!(receipt.local_state_digest_before.len(), 64);
    assert_eq!(receipt.local_state_digest_after.len(), 64);
    assert_ne!(
        receipt.local_state_digest_before,
        receipt.local_state_digest_after
    );
    assert_eq!(receipt.receipt_digest.len(), 64);
    assert!(validate_deployment_root_verification_receipt(&receipt).is_ok());

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn verify_registered_deployment_root_reverifies_same_root_without_state_write() {
    let (root, _) = demo_unverified_registered_root_check("canic-root-verify-reverify");
    let mut verified_state = read_deployment_install_state(&root, "local", "demo-local")
        .expect("read state")
        .expect("state exists");
    verified_state.root_verification = RootVerificationStatus::Verified;
    verified_state.updated_at_unix_secs = 100;
    write_install_state(&root, "local", &verified_state).expect("write verified state");
    let check = demo_registered_root_check_from_state(&root);
    let state_before = read_deployment_install_state(&root, "local", "demo-local")
        .expect("read before")
        .expect("state before");

    let receipt = verify_registered_deployment_root(VerifyDeploymentRootOptions {
        deployment_name: "demo-local".to_string(),
        network: "local".to_string(),
        deployment_check: check,
        verified_at_unix_secs: Some(200),
        icp_root: Some(root.clone()),
    })
    .expect("reverify registered root");
    let state_after = read_deployment_install_state(&root, "local", "demo-local")
        .expect("read after")
        .expect("state after");

    assert_eq!(
        state_after.root_verification,
        RootVerificationStatus::Verified
    );
    assert_eq!(
        state_after.updated_at_unix_secs,
        state_before.updated_at_unix_secs
    );
    assert_eq!(
        receipt.state_transition,
        crate::deployment_truth::DeploymentRootVerificationStateTransitionV1::NoStateChange
    );
    assert_eq!(receipt.verified_at_unix_secs, 200);
    assert_eq!(
        receipt.local_state_digest_before,
        receipt.local_state_digest_after
    );
    assert!(validate_deployment_root_verification_receipt(&receipt).is_ok());

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn verify_registered_deployment_root_rejects_verified_root_replacement() {
    let (root, mut check) = demo_unverified_registered_root_check("canic-root-verify-replace");
    let mut verified_state = read_deployment_install_state(&root, "local", "demo-local")
        .expect("read state")
        .expect("state exists");
    verified_state.root_verification = RootVerificationStatus::Verified;
    verified_state.updated_at_unix_secs = 100;
    write_install_state(&root, "local", &verified_state).expect("write verified state");
    check.report.hard_failures.clear();
    check.report.status = SafetyStatusV1::Safe;
    let observed_root = check
        .inventory
        .observed_root
        .as_mut()
        .expect("observed root");
    observed_root.root_principal = "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string();
    observed_root.observed_canister_id = "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string();

    let err = verify_registered_deployment_root(VerifyDeploymentRootOptions {
        deployment_name: "demo-local".to_string(),
        network: "local".to_string(),
        deployment_check: check,
        verified_at_unix_secs: Some(200),
        icp_root: Some(root.clone()),
    })
    .expect_err("root replacement must fail");
    let state_after = read_deployment_install_state(&root, "local", "demo-local")
        .expect("read after")
        .expect("state after");

    assert!(
        err.to_string()
            .contains("deployment root verification failed")
    );
    assert_eq!(
        state_after.root_canister_id,
        verified_state.root_canister_id
    );
    assert_eq!(
        state_after.root_verification,
        RootVerificationStatus::Verified
    );
    assert_eq!(state_after.updated_at_unix_secs, 100);

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn verify_registered_deployment_root_rejects_local_state_only_evidence() {
    let (root, mut check) = demo_unverified_registered_root_check("canic-root-verify-local-only");
    let observed_root = check
        .inventory
        .observed_root
        .as_mut()
        .expect("observed root");
    observed_root.observation_source = DeploymentRootObservationSourceV1::LocalDeploymentState;

    let err = verify_registered_deployment_root(VerifyDeploymentRootOptions {
        deployment_name: "demo-local".to_string(),
        network: "local".to_string(),
        deployment_check: check,
        verified_at_unix_secs: Some(100),
        icp_root: Some(root.clone()),
    })
    .expect_err("local-state-only evidence must not verify root");
    let state = read_deployment_install_state(&root, "local", "demo-local")
        .expect("read state")
        .expect("state exists");

    assert!(
        err.to_string()
            .contains("deployment root verification failed")
    );
    assert_eq!(state.root_verification, RootVerificationStatus::NotVerified);

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn verified_root_state_writes_stay_on_explicit_install_or_verify_paths() {
    let install_source = include_str!("../../install_state/mod.rs");
    let registration_source = include_str!("../../deployment_registration/mod.rs");

    assert_eq!(
        install_source
            .matches("root_verification: RootVerificationStatus::Verified")
            .count(),
        1,
        "only install-created state may initialize verified root state"
    );
    assert_eq!(
        registration_source
            .matches("root_verification = RootVerificationStatus::Verified")
            .count(),
        1,
        "only explicit root verification may promote existing registered state"
    );
}

#[test]
fn verify_registered_root_validates_and_writes_before_receipt() {
    let source = include_str!("../../deployment_registration/mod.rs");
    let start = source
        .find("pub fn verify_registered_deployment_root(")
        .expect("verify function start");
    let end = source[start..]
        .find("fn registered_deployment_release_set_manifest_path(")
        .map(|offset| start + offset)
        .expect("verify function end");
    let body = &source[start..end];

    let validate_report = body
        .find("validate_deployment_root_verification_report(&report)?")
        .expect("report validation");
    let state_assignment = body
        .find("verified_state.root_verification = RootVerificationStatus::Verified")
        .expect("verified state assignment");
    let compare_and_swap_write = body
        .find("write_verified_root_state_if_unchanged(")
        .expect("compare-and-swap write");
    let receipt_creation = body
        .find("root_verification_receipt_from_report(")
        .expect("receipt creation");

    assert!(
        validate_report < state_assignment,
        "root verification must validate deployment-truth evidence before changing local state"
    );
    assert!(
        state_assignment < compare_and_swap_write,
        "root verification must prepare verified state before the guarded write"
    );
    assert!(
        compare_and_swap_write < receipt_creation,
        "root verification must create receipts only after the guarded write"
    );
    assert!(
        !body.contains("write_install_state("),
        "root verification must write through write_verified_root_state_if_unchanged"
    );
}

#[test]
fn verify_registered_deployment_root_rejects_state_digest_race() {
    let root = temp_dir("canic-root-verify-state-race");
    let state = sample_install_state(&root, "demo-local", "demo");
    write_install_state(&root, "local", &state).expect("write state");
    let mut changed = state.clone();
    changed.updated_at_unix_secs = 99;

    let err = write_verified_root_state_if_unchanged(&root, "local", &changed, "not-current")
        .expect_err("stale digest must fail closed");
    let stored = read_deployment_install_state(&root, "local", "demo-local")
        .expect("read state")
        .expect("state exists");

    assert!(
        err.to_string()
            .contains("deployment root verification state changed before write")
    );
    assert_eq!(stored.updated_at_unix_secs, state.updated_at_unix_secs);

    fs::remove_dir_all(root).expect("clean temp dir");
}
