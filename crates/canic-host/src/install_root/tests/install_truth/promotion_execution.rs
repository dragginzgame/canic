use super::*;
use canic_core::cdk::utils::hash::wasm_hash_hex;
use flate2::{Compression, GzBuilder};
use std::io::Write;

const MINIMAL_WASM: &[u8] = b"\0asm\x01\0\0\0";
const OTHER_WASM: &[u8] = b"\0asm\x01\0\0\0\x00\x01\x00";

#[test]
fn install_plan_artifact_validation_rejects_missing_root_wasm_before_mutation() {
    let (root, check) = demo_install_deployment_truth_check("canic-install-plan-missing-root-wasm");

    let Err(_) = prepare_plan_artifacts_with_phase(&check.plan, &root, "local") else {
        panic!("missing root wasm should fail before install mutation");
    };
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn supplied_gzip_materializes_one_verified_pair_and_detaches_from_source() {
    let (root, check) =
        demo_install_deployment_truth_check("canic-install-plan-gzip-materialization");
    let source = root.join("inputs/root.wasm.gz");
    let gzip = gzip_wasm(MINIMAL_WASM);
    write_test_artifact(&source, &gzip);
    let plan = root_only_plan(
        check.plan,
        None,
        Some("inputs/root.wasm.gz"),
        Some(wasm_hash_hex(MINIMAL_WASM)),
        Some(wasm_hash_hex(&gzip)),
    );

    let prepared =
        PreparedPlanArtifacts::materialize(&plan, &root, "local").expect("materialize gzip source");
    let root_wasm = prepared
        .verified_root_wasm_path()
        .expect("verify canonical root Wasm");
    assert_eq!(fs::read(&root_wasm).expect("read root Wasm"), MINIMAL_WASM);
    assert_eq!(
        prepared.plan().role_artifacts[0].wasm_path.as_deref(),
        Some(root_wasm.to_string_lossy().as_ref())
    );

    write_test_artifact(&source, &gzip_wasm(OTHER_WASM));
    assert_eq!(
        fs::read(
            prepared
                .verified_root_wasm_path()
                .expect("source replacement must not alter prepared bytes")
        )
        .expect("read prepared Wasm"),
        MINIMAL_WASM
    );
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn supplied_raw_wasm_derives_the_canonical_gzip_representation() {
    let (root, check) =
        demo_install_deployment_truth_check("canic-install-plan-raw-materialization");
    let source = root.join("inputs/root.wasm");
    write_test_artifact(&source, MINIMAL_WASM);
    let plan = root_only_plan(
        check.plan,
        Some("inputs/root.wasm"),
        None,
        Some(wasm_hash_hex(MINIMAL_WASM)),
        None,
    );

    let prepared =
        PreparedPlanArtifacts::materialize(&plan, &root, "local").expect("materialize raw source");
    let artifact = &prepared.plan().role_artifacts[0];
    let gzip_path = artifact.wasm_gz_path.as_ref().expect("canonical gzip path");

    assert!(Path::new(gzip_path).is_file());
    assert_eq!(
        artifact.wasm_gz_sha256.as_deref(),
        Some(wasm_hash_hex(&fs::read(gzip_path).expect("read gzip")).as_str())
    );
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn prepared_root_revalidation_rejects_post_gate_byte_replacement() {
    let (root, check) = demo_install_deployment_truth_check("canic-install-plan-root-revalidation");
    let gzip = gzip_wasm(MINIMAL_WASM);
    write_test_artifact(&root.join("inputs/root.wasm.gz"), &gzip);
    let plan = root_only_plan(
        check.plan,
        None,
        Some("inputs/root.wasm.gz"),
        Some(wasm_hash_hex(MINIMAL_WASM)),
        Some(wasm_hash_hex(&gzip)),
    );
    let prepared =
        PreparedPlanArtifacts::materialize(&plan, &root, "local").expect("materialize root source");
    let root_wasm = prepared
        .verified_root_wasm_path()
        .expect("verify prepared root");

    fs::write(root_wasm, OTHER_WASM).expect("replace prepared root bytes");
    let error = prepared
        .verified_root_wasm_path()
        .expect_err("post-gate replacement must reject");

    std::assert_matches!(
        error,
        PlanArtifactError::RepresentationMismatch { role } if role == "root"
    );
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn prepared_release_manifest_uses_the_same_verified_nonroot_bytes() {
    let (root, check) = demo_install_deployment_truth_check("canic-install-plan-release-manifest");
    let root_gzip = gzip_wasm(MINIMAL_WASM);
    let app_gzip = gzip_wasm(OTHER_WASM);
    write_test_artifact(&root.join("inputs/root.wasm.gz"), &root_gzip);
    write_test_artifact(&root.join("inputs/app.wasm.gz"), &app_gzip);
    let mut plan = check.plan;
    plan.role_artifacts
        .retain(|artifact| artifact.role == "root");
    let mut app_artifact = plan.role_artifacts[0].clone();
    app_artifact.role = "app".to_string();
    plan.role_artifacts.push(app_artifact);
    configure_artifact(
        plan.role_artifacts
            .iter_mut()
            .find(|artifact| artifact.role == "root")
            .expect("root artifact"),
        None,
        Some("inputs/root.wasm.gz"),
        Some(wasm_hash_hex(MINIMAL_WASM)),
        Some(wasm_hash_hex(&root_gzip)),
    );
    configure_artifact(
        plan.role_artifacts
            .iter_mut()
            .find(|artifact| artifact.role == "app")
            .expect("app artifact"),
        None,
        Some("inputs/app.wasm.gz"),
        Some(wasm_hash_hex(OTHER_WASM)),
        Some(wasm_hash_hex(&app_gzip)),
    );

    let prepared = PreparedPlanArtifacts::materialize(&plan, &root, "local")
        .expect("materialize release artifacts");
    let manifest_path = prepared
        .emit_release_set_manifest()
        .expect("emit verified release manifest");
    let manifest = crate::release_set::load_root_release_set_manifest(&manifest_path)
        .expect("load release manifest");

    assert_eq!(manifest.entries.len(), 1);
    assert_eq!(manifest.entries[0].role, "app");
    assert_eq!(
        manifest.entries[0].payload_sha256_hex,
        wasm_hash_hex(&app_gzip)
    );
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn supplied_artifact_pair_rejects_different_wasm_representations() {
    let (root, check) = demo_install_deployment_truth_check("canic-install-plan-pair-mismatch");
    write_test_artifact(&root.join("inputs/root.wasm"), MINIMAL_WASM);
    let gzip = gzip_wasm(OTHER_WASM);
    write_test_artifact(&root.join("inputs/root.wasm.gz"), &gzip);
    let plan = root_only_plan(
        check.plan,
        Some("inputs/root.wasm"),
        Some("inputs/root.wasm.gz"),
        Some(wasm_hash_hex(MINIMAL_WASM)),
        Some(wasm_hash_hex(&gzip)),
    );

    let error = PreparedPlanArtifacts::materialize(&plan, &root, "local")
        .expect_err("different raw and gzip bytes must reject");

    std::assert_matches!(
        error,
        PlanArtifactError::RepresentationMismatch { role } if role == "root"
    );
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn supplied_artifact_rejects_traversal_and_digest_drift() {
    let (root, check) = demo_install_deployment_truth_check("canic-install-plan-path-rejection");
    let traversal = root_only_plan(
        check.plan.clone(),
        Some("../root.wasm"),
        None,
        Some(wasm_hash_hex(MINIMAL_WASM)),
        None,
    );
    let traversal_error = PreparedPlanArtifacts::materialize(&traversal, &root, "local")
        .expect_err("parent traversal must reject");
    std::assert_matches!(traversal_error, PlanArtifactError::UnsafePath { .. });

    let gzip = gzip_wasm(MINIMAL_WASM);
    write_test_artifact(&root.join("inputs/root.wasm.gz"), &gzip);
    let drifted = root_only_plan(
        check.plan,
        None,
        Some("inputs/root.wasm.gz"),
        Some(wasm_hash_hex(MINIMAL_WASM)),
        Some("0".repeat(64)),
    );
    let digest_error = PreparedPlanArtifacts::materialize(&drifted, &root, "local")
        .expect_err("digest drift must reject");
    std::assert_matches!(
        digest_error,
        PlanArtifactError::DigestMismatch { role, kind: "gzip Wasm", .. } if role == "root"
    );
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn supplied_artifact_rejects_unpinned_and_duplicate_roles() {
    let (root, check) =
        demo_install_deployment_truth_check("canic-install-plan-structural-rejection");
    write_test_artifact(&root.join("inputs/root.wasm"), MINIMAL_WASM);
    let mut unpinned = root_only_plan(
        check.plan.clone(),
        Some("inputs/root.wasm"),
        None,
        None,
        None,
    );
    unpinned.role_artifacts[0].installed_module_hash = Some(wasm_hash_hex(MINIMAL_WASM));
    let pin_error = PreparedPlanArtifacts::materialize(&unpinned, &root, "local")
        .expect_err("installed deployment state must not substitute for an artifact pin");
    std::assert_matches!(
        pin_error,
        PlanArtifactError::MissingDigestPin { role, kind: "raw Wasm" } if role == "root"
    );

    let gzip = gzip_wasm(MINIMAL_WASM);
    write_test_artifact(&root.join("inputs/root.wasm.gz"), &gzip);
    let gzip_unpinned = root_only_plan(
        check.plan.clone(),
        None,
        Some("inputs/root.wasm.gz"),
        Some(wasm_hash_hex(MINIMAL_WASM)),
        None,
    );
    let gzip_pin_error = PreparedPlanArtifacts::materialize(&gzip_unpinned, &root, "local")
        .expect_err("gzip source must pin its exact compressed bytes");
    std::assert_matches!(
        gzip_pin_error,
        PlanArtifactError::MissingDigestPin { role, kind: "gzip Wasm" } if role == "root"
    );

    let mut duplicate = root_only_plan(
        check.plan,
        Some("inputs/root.wasm"),
        None,
        Some(wasm_hash_hex(MINIMAL_WASM)),
        None,
    );
    duplicate
        .role_artifacts
        .push(duplicate.role_artifacts[0].clone());
    let duplicate_error = PreparedPlanArtifacts::materialize(&duplicate, &root, "local")
        .expect_err("duplicate role must reject");
    std::assert_matches!(
        duplicate_error,
        PlanArtifactError::DuplicateRole { role } if role == "root"
    );
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[cfg(unix)]
#[test]
fn supplied_artifact_rejects_symlinked_sources() {
    let (root, check) = demo_install_deployment_truth_check("canic-install-plan-symlink-rejection");
    let source = root.join("inputs/root.wasm");
    write_test_artifact(&source, MINIMAL_WASM);
    let link = root.join("inputs/linked.wasm");
    std::os::unix::fs::symlink(&source, &link).expect("create source symlink");
    let plan = root_only_plan(
        check.plan,
        Some("inputs/linked.wasm"),
        None,
        Some(wasm_hash_hex(MINIMAL_WASM)),
        None,
    );

    let error = PreparedPlanArtifacts::materialize(&plan, &root, "local")
        .expect_err("symlinked source must reject");

    std::assert_matches!(error, PlanArtifactError::UnsafePath { .. });
    fs::remove_dir_all(root).expect("clean temp dir");
}

#[cfg(unix)]
#[test]
fn supplied_artifact_rejects_symlinked_canonical_environment_root() {
    let (root, check) = demo_install_deployment_truth_check("canic-install-plan-target-symlink");
    let gzip = gzip_wasm(MINIMAL_WASM);
    write_test_artifact(&root.join("inputs/root.wasm.gz"), &gzip);
    fs::remove_dir_all(root.join(".icp/local")).expect("remove original environment root");
    let escaped = root.join("escaped-environment");
    fs::create_dir_all(&escaped).expect("create escaped environment root");
    std::os::unix::fs::symlink(&escaped, root.join(".icp/local"))
        .expect("create environment symlink");
    let plan = root_only_plan(
        check.plan,
        None,
        Some("inputs/root.wasm.gz"),
        Some(wasm_hash_hex(MINIMAL_WASM)),
        Some(wasm_hash_hex(&gzip)),
    );

    let error = PreparedPlanArtifacts::materialize(&plan, &root, "local")
        .expect_err("symlinked canonical target must reject");

    std::assert_matches!(error, PlanArtifactError::UnsafePath { .. });
    fs::remove_dir_all(root).expect("clean temp dir");
}

fn root_only_plan(
    mut plan: crate::deployment_truth::DeploymentPlanV1,
    wasm_path: Option<&str>,
    wasm_gz_path: Option<&str>,
    wasm_sha256: Option<String>,
    wasm_gz_sha256: Option<String>,
) -> crate::deployment_truth::DeploymentPlanV1 {
    plan.role_artifacts
        .retain(|artifact| artifact.role == "root");
    let artifact = plan.role_artifacts.first_mut().expect("root artifact");
    configure_artifact(
        artifact,
        wasm_path,
        wasm_gz_path,
        wasm_sha256,
        wasm_gz_sha256,
    );
    plan
}

fn configure_artifact(
    artifact: &mut crate::deployment_truth::RoleArtifactV1,
    wasm_path: Option<&str>,
    wasm_gz_path: Option<&str>,
    wasm_sha256: Option<String>,
    wasm_gz_sha256: Option<String>,
) {
    artifact.wasm_path = wasm_path.map(str::to_string);
    artifact.wasm_gz_path = wasm_gz_path.map(str::to_string);
    artifact.wasm_sha256 = wasm_sha256;
    artifact.wasm_gz_sha256.clone_from(&wasm_gz_sha256);
    artifact.observed_wasm_gz_file_sha256 = wasm_gz_sha256;
}

fn gzip_wasm(wasm: &[u8]) -> Vec<u8> {
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder.write_all(wasm).expect("compress test Wasm");
    encoder.finish().expect("finish test gzip")
}

fn write_test_artifact(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().expect("artifact parent")).expect("create artifact parent");
    fs::write(path, bytes).expect("write test artifact");
}

#[test]
fn install_writes_artifact_promotion_execution_receipt_for_promotion_plan() {
    let (root, mut check) =
        demo_install_deployment_truth_check("canic-install-promotion-execution-receipt");
    let artifact = check
        .plan
        .role_artifacts
        .iter_mut()
        .find(|artifact| artifact.role == "root")
        .expect("root artifact");
    artifact.wasm_sha256 = Some(sample_sha256("d"));
    artifact.wasm_gz_sha256 = Some(sample_sha256("a"));
    artifact.observed_wasm_gz_file_sha256 = Some(sample_sha256("a"));
    artifact.canonical_embedded_config_sha256 = Some(sample_sha256("c"));
    let promotion_plan = sample_artifact_promotion_plan_for_install(&check);
    let execution_context = current_install_execution_context(&root, &root, "local");
    let options = InstallRootOptions {
        root_canister: "root".to_string(),
        root_build_target: "root".to_string(),
        environment: "local".to_string(),
        deployment_name: None,
        icp_root: Some(root.clone()),
        build_profile: Some(CanisterBuildProfile::Fast),
        ready_timeout_seconds: 30,
        config_path: Some("apps/demo/canic.toml".to_string()),
        expected_fleet: Some("demo".to_string()),
        interactive_config_selection: false,
        deployment_plan_override: Some(check.plan.clone()),
        artifact_promotion_plan_override: Some(promotion_plan.clone()),
    };
    let latest_deployment_receipt_before =
        latest_deployment_truth_receipt_path_from_root(&root, "local", "demo").ok();

    let path = write_artifact_promotion_execution_receipt_for_install(
        &options,
        &root,
        "local",
        "demo",
        &check,
        &execution_context,
    )
    .expect("promotion execution receipt write")
    .expect("promotion execution receipt path");
    let receipt: ArtifactPromotionExecutionReceiptV1 =
        serde_json::from_slice(&fs::read(&path).expect("read promotion receipt"))
            .expect("decode promotion receipt");

    assert!(
        path.display()
            .to_string()
            .contains("artifact-promotion-execution-receipts")
    );
    assert_eq!(receipt.artifact_promotion_plan_id, promotion_plan.plan_id);
    assert_eq!(receipt.promoted_plan_id, check.plan.plan_id);
    assert_eq!(receipt.deployment_receipt.plan_id, check.plan.plan_id);
    assert_eq!(receipt.roles.len(), 1);
    assert_eq!(receipt.roles[0].role, "root");
    assert_eq!(
        latest_deployment_truth_receipt_path_from_root(&root, "local", "demo").ok(),
        latest_deployment_receipt_before,
        "promotion wrapper emission must not update ordinary deployment receipt discovery"
    );
    fs::remove_dir_all(root).expect("clean temp dir");
}
