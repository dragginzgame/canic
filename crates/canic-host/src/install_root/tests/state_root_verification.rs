use super::*;

#[test]
fn install_rejects_config_identity_mismatch() {
    let err =
        validate_expected_fleet_name(Some("demo"), "test", Path::new("fleets/demo/canic.toml"))
            .expect_err("mismatched fleet identity should fail");

    assert!(err.to_string().contains(
        "install requested fleet demo, but fleets/demo/canic.toml declares [fleet].name = \"test\""
    ));
}

#[test]
fn deployment_state_path_is_scoped_by_network() {
    assert_eq!(
        deployment_install_state_path(&PathBuf::from("/tmp/canic-project"), "local", "demo"),
        PathBuf::from("/tmp/canic-project/.canic/local/deployments/demo.json")
    );
}

#[test]
fn install_state_round_trips_from_project_state_dir() {
    let root = temp_dir("canic-install-state");
    let state = sample_install_state(&root, "demo", "demo");

    let path = write_install_state(&root, "local", &state).expect("write state");
    let named = read_deployment_install_state(&root, "local", "demo")
        .expect("read named deployment")
        .expect("named deployment exists");

    assert_eq!(path, root.join(".canic/local/deployments/demo.json"));
    assert_eq!(named, state);

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_state_rejects_stale_duplicate_fleet_field() {
    let root = temp_dir("canic-install-state-stale-fleet-field");
    let path = deployment_install_state_path(&root, "local", "demo-local");
    fs::create_dir_all(path.parent().expect("state parent")).expect("create state dir");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({
            "schema_version": INSTALL_STATE_SCHEMA_VERSION,
            "fleet": "demo",
            "deployment_name": "demo-local",
            "fleet_template": "demo",
            "created_at_unix_secs": 42,
            "updated_at_unix_secs": 42,
            "network": "local",
            "root_target": "root",
            "root_canister_id": "uxrrr-q7777-77774-qaaaq-cai",
            "root_verification": "verified",
            "root_build_target": "root",
            "workspace_root": root.display().to_string(),
            "icp_root": root.display().to_string(),
            "config_path": root.join("fleets/demo/canic.toml").display().to_string(),
            "release_set_manifest_path": root
                .join(".icp/local/canisters/root/root.release-set.json")
                .display()
                .to_string()
        }))
        .expect("encode stale state"),
    )
    .expect("write stale state");

    let err = read_deployment_install_state(&root, "local", "demo-local")
        .expect_err("stale duplicate fleet field must fail closed");
    let message = err.to_string();

    assert!(message.contains("unknown field `fleet`"));

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_state_rejects_stale_installed_timestamp_field() {
    let root = temp_dir("canic-install-state-stale-installed-at");
    let path = deployment_install_state_path(&root, "local", "demo-local");
    fs::create_dir_all(path.parent().expect("state parent")).expect("create state dir");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({
            "schema_version": INSTALL_STATE_SCHEMA_VERSION,
            "deployment_name": "demo-local",
            "fleet_template": "demo",
            "installed_at_unix_secs": 42,
            "network": "local",
            "root_target": "root",
            "root_canister_id": "uxrrr-q7777-77774-qaaaq-cai",
            "root_verification": "verified",
            "root_build_target": "root",
            "workspace_root": root.display().to_string(),
            "icp_root": root.display().to_string(),
            "config_path": root.join("fleets/demo/canic.toml").display().to_string(),
            "release_set_manifest_path": root
                .join(".icp/local/canisters/root/root.release-set.json")
                .display()
                .to_string()
        }))
        .expect("encode stale state"),
    )
    .expect("write stale state");

    let err = read_deployment_install_state(&root, "local", "demo-local")
        .expect_err("stale installed timestamp field must fail closed");
    let message = err.to_string();

    assert!(message.contains("unknown field `installed_at_unix_secs`"));

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn deploy_register_writes_minimal_unverified_deployment_state() {
    let root = temp_dir("canic-register-state");
    let path = register_deployment_state(RegisterDeploymentStateOptions {
        deployment_name: "demo-local".to_string(),
        fleet_template: "demo".to_string(),
        root_canister_id: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
        network: "local".to_string(),
        allow_unverified: true,
        icp_root: Some(root.clone()),
        workspace_root: Some(root.clone()),
    })
    .expect("register deployment state");
    let state = read_deployment_install_state(&root, "local", "demo-local")
        .expect("read registered state")
        .expect("state exists");

    assert_eq!(path, root.join(".canic/local/deployments/demo-local.json"));
    assert_eq!(state.deployment_name, "demo-local");
    assert_eq!(state.fleet_template, "demo");
    assert_eq!(state.root_canister_id, "uxrrr-q7777-77774-qaaaq-cai");
    assert_eq!(state.root_verification, RootVerificationStatus::NotVerified);
    assert_eq!(state.created_at_unix_secs, state.updated_at_unix_secs);
    assert!(state.config_path.ends_with("fleets/demo/canic.toml"));

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn deploy_register_requires_explicit_unverified_acknowledgement() {
    let root = temp_dir("canic-register-state-requires-ack");
    let err = register_deployment_state(RegisterDeploymentStateOptions {
        deployment_name: "demo-local".to_string(),
        fleet_template: "demo".to_string(),
        root_canister_id: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
        network: "local".to_string(),
        allow_unverified: false,
        icp_root: Some(root.clone()),
        workspace_root: Some(root.clone()),
    })
    .expect_err("registration without acknowledgement must fail");

    assert!(err.to_string().contains("--allow-unverified"));

    if root.exists() {
        fs::remove_dir_all(root).expect("clean temp dir");
    }
}

#[test]
fn unverified_registered_root_is_not_used_as_plan_authority() {
    let root = temp_dir("canic-register-unverified-plan");
    let workspace_root = root.join("workspace");
    let icp_root = root.join("icp");
    let config_dir = workspace_root.join("fleets");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(
        config_dir.join("canic.toml"),
        r#"
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.project_registry]
kind = "canister"
package = "project_registry"

[roles.oracle_pokemon]
kind = "canister"
package = "oracle_pokemon"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[roles.scale_hub]
kind = "canister"
package = "scale_hub"

[roles.scale_replica]
kind = "canister"
package = "scale"

[roles.role_baseline]
kind = "canister"
package = "role_baseline"

[roles.worker]
kind = "canister"
package = "worker"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    register_deployment_state(RegisterDeploymentStateOptions {
        deployment_name: "demo-local".to_string(),
        fleet_template: "demo".to_string(),
        root_canister_id: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
        network: "local".to_string(),
        allow_unverified: true,
        icp_root: Some(icp_root.clone()),
        workspace_root: Some(workspace_root.clone()),
    })
    .expect("register deployment state");

    let plan = crate::deployment_truth::build_local_deployment_plan(
        &crate::deployment_truth::LocalDeploymentPlanRequest {
            deployment_name: "demo-local".to_string(),
            network: "local".to_string(),
            workspace_root,
            icp_root,
            config_path: None,
            runtime_variant: "local".to_string(),
            build_profile: "fast".to_string(),
        },
    );

    assert_eq!(plan.trust_domain.root_trust_anchor, None);
    assert!(plan.unresolved_assumptions.iter().any(|assumption| {
        assumption.key == "local_state.unverified_root_canister_id"
            && assumption
                .description
                .contains("root verification is NotVerified")
    }));

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn unverified_registered_root_blocks_install_truth_gate() {
    let root = temp_dir("canic-register-unverified-gate");
    let workspace_root = root.join("workspace");
    let icp_root = root.join("icp");
    let config_path = workspace_root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.project_registry]
kind = "canister"
package = "project_registry"

[roles.oracle_pokemon]
kind = "canister"
package = "oracle_pokemon"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[roles.scale_hub]
kind = "canister"
package = "scale_hub"

[roles.scale_replica]
kind = "canister"
package = "scale"

[roles.role_baseline]
kind = "canister"
package = "role_baseline"

[roles.worker]
kind = "canister"
package = "worker"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&icp_root, "root", b"root-artifact");
    register_deployment_state(RegisterDeploymentStateOptions {
        deployment_name: "demo-local".to_string(),
        fleet_template: "demo".to_string(),
        root_canister_id: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
        network: "local".to_string(),
        allow_unverified: true,
        icp_root: Some(icp_root.clone()),
        workspace_root: Some(workspace_root.clone()),
    })
    .expect("register deployment state");
    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        deployment_name: Some("demo-local".to_string()),
        icp_root: Some(icp_root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some(config_path.display().to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: None,
        artifact_promotion_plan_override: None,
    };

    let check = current_install_deployment_truth_check_at(
        &options,
        &workspace_root,
        &icp_root,
        &config_path,
        "demo-local",
        "2026-05-22T00:00:00Z".to_string(),
    )
    .expect("deployment truth check");
    let err = enforce_install_deployment_truth_gate(&check)
        .expect_err("unverified registered root must block mutation");

    assert!(check.report.hard_failures.iter().any(|finding| {
        finding.code == "unverified_deployment_root"
            && finding.subject.as_deref() == Some("local_state.unverified_root_canister_id")
    }));
    assert!(err.to_string().contains("unverified_deployment_root"));

    fs::remove_dir_all(root).expect("clean temp dir");
}

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
    let source = include_str!("../mod.rs");

    assert_eq!(
        source
            .matches("root_verification: RootVerificationStatus::Verified")
            .count(),
        1,
        "only install-created state may initialize verified root state"
    );
    assert_eq!(
        source
            .matches("root_verification = RootVerificationStatus::Verified")
            .count(),
        1,
        "only explicit root verification may promote existing registered state"
    );
}

#[test]
fn verify_registered_root_validates_and_writes_before_receipt() {
    let source = include_str!("../mod.rs");
    let start = source
        .find("pub fn verify_registered_deployment_root(")
        .expect("verify function start");
    let end = source[start..]
        .find("struct PreparedInstallTruth")
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

#[test]
fn install_truth_check_uses_deployment_state_config_for_target_named_commands() {
    let root = temp_dir("canic-deploy-target-state-config");
    let config_path = root.join("fleets/demo/canic.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[roles.project_registry]
kind = "canister"
package = "project_registry"

[roles.oracle_pokemon]
kind = "canister"
package = "oracle_pokemon"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[roles.scale_hub]
kind = "canister"
package = "scale_hub"

[roles.scale_replica]
kind = "canister"
package = "scale"

[roles.role_baseline]
kind = "canister"
package = "role_baseline"

[roles.worker]
kind = "canister"
package = "worker"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"
"#,
    )
    .expect("write config");
    write_wasm_gz_artifact(&root, "root", b"root-artifact");
    let state = sample_install_state(&root, "demo-local", "demo");
    write_install_state(&root, "local", &state).expect("write deployment state");
    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        network: "local".to_string(),
        deployment_name: Some("demo-local".to_string()),
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: None,
        expected_fleet: None,
        interactive_config_selection: false,
        deployment_plan_override: None,
        artifact_promotion_plan_override: None,
    };

    let check = check_install_deployment_truth(&options, "2026-05-22T00:00:00Z")
        .expect("deployment truth check");

    assert_eq!(check.plan.deployment_identity.deployment_name, "demo-local");
    assert_eq!(check.plan.fleet_template, "demo");
    assert_eq!(
        check.plan.trust_domain.root_trust_anchor.as_deref(),
        Some("uxrrr-q7777-77774-qaaaq-cai")
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn install_truth_state_write_receipt_records_local_state_path() {
    let (root, check) = demo_install_deployment_truth_check("canic-install-state-receipt");
    let state = sample_install_state(&root, "demo", "demo");
    let execution_context = current_install_execution_context(&root, &root, "local");
    let scope = InstallReceiptScope {
        icp_root: &root,
        network: "local",
        deployment_name: "demo",
        check: &check,
        execution_context: Some(&execution_context),
    };

    let state_path = write_install_state_with_deployment_truth_receipt(scope, "local", &state)
        .expect("write install state and receipt");
    let receipt_dir = root.join(".canic/local/deployment-receipts/demo");
    let receipt = fs::read_dir(&receipt_dir)
        .expect("read receipts")
        .map(|entry| {
            let path = entry.expect("receipt entry").path();
            serde_json::from_slice::<DeploymentReceiptV1>(
                &fs::read(path).expect("read receipt JSON"),
            )
            .expect("decode receipt")
        })
        .find(|receipt| receipt.operation_id.ends_with(":write_install_state"))
        .expect("write install state receipt");

    assert_eq!(state_path, root.join(".canic/local/deployments/demo.json"));
    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(receipt.phase_receipts[0].phase, "write_install_state");
    assert!(
        receipt.phase_receipts[0]
            .verified_postcondition
            .evidence
            .contains(&format!("install_state:{}", state_path.display()))
    );
    assert!(
        receipt.phase_receipts[0]
            .verified_postcondition
            .evidence
            .contains(&"deployment:demo".to_string())
    );
    assert!(
        receipt.phase_receipts[0]
            .verified_postcondition
            .evidence
            .contains(&"fleet_template:demo".to_string())
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn deployment_state_allows_distinct_targets_that_share_root() {
    let root = temp_dir("canic-install-state-targets");
    let demo = sample_install_state(&root, "demo-local", "demo");
    let test = sample_install_state(&root, "demo-staging", "demo");

    write_install_state(&root, "local", &demo).expect("write demo state");
    write_install_state(&root, "local", &test).expect("write test state");

    assert_eq!(
        read_deployment_install_state(&root, "local", "demo-local")
            .expect("read demo")
            .expect("demo state exists"),
        demo
    );
    assert_eq!(
        read_deployment_install_state(&root, "local", "demo-staging")
            .expect("read test")
            .expect("test state exists"),
        test
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn legacy_fleet_state_is_rejected_as_deployment_truth() {
    let root = temp_dir("canic-install-state-legacy");
    let legacy_path = legacy_fleet_install_state_path(&root, "local", "demo");
    fs::create_dir_all(legacy_path.parent().expect("legacy parent")).expect("create legacy dir");
    fs::write(&legacy_path, b"{}").expect("write legacy state");

    let err = read_deployment_install_state(&root, "local", "demo")
        .expect_err("legacy fleet state must fail closed");
    let message = err.to_string();

    assert!(message.contains("legacy fleet install state found"));
    assert!(message.contains(
        "canic deploy register demo --fleet-template <fleet-template> --root <principal> --allow-unverified"
    ));
    assert!(message.contains("canic install <fleet-template>"));
    assert!(message.contains(".canic/local/fleets/demo.json"));

    fs::remove_dir_all(root).expect("clean temp dir");
}
