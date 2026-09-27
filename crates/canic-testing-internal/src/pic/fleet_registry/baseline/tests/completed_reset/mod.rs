//! Qualify a completed estate through current receipts, current reset and recovery.
//!
//! All remote effects use production adapters against disposable PocketIC canisters.

mod capacity_import;

use super::*;
use canic_core::cdk::utils::hash::sha256_hex;
use canic_host::fleet_ensure::{
    ops::{EnsurePaths, read_state, retained_contract},
    workflow::{completed_preparation, completed_reset},
};
use std::collections::BTreeMap;

const CLI_ARGUMENTS: &str = "CANIC_COMPLETED_RESET_CLI_ARGUMENTS";
const CLI_TEST: &str = "pic::fleet_registry::baseline::tests::completed_reset::completed_estate_reset_recovers_and_replays";

#[test]
pub(super) fn completed_estate_reset_recovers_and_replays() {
    if let Ok(arguments) = std::env::var(CLI_ARGUMENTS) {
        let arguments: Vec<String> = serde_json::from_str(&arguments).unwrap();
        canic_cli::run(arguments.into_iter().map(std::ffi::OsString::from)).unwrap();
        return;
    }
    assert_literal_zero_host_journey(FundingJourney::CompletedReset, RETAINED_ESTATE_WORKLOADS);
}

fn paths(input: &ReinstallJourney<'_>) -> EnsurePaths {
    EnsurePaths::under(input.adapter_root, "local", &input.desired.fleet)
}

pub(super) fn retained_desired(mut desired: DesiredFleet) -> DesiredFleet {
    // Declare supplied Ready capacity before reviewing and executing the source.
    for root in &mut desired.bootstrap.as_mut().unwrap().roots {
        root.canister_pool_imports = desired
            .canisters
            .iter()
            .filter(|canister| {
                canister.kind == canic_host::fleet_ensure::model::DesiredCanisterKind::Pool
                    && canister.parent.as_ref() == Some(&root.root)
            })
            .map(|canister| canister.name.clone())
            .collect();
    }
    desired
}

pub(super) fn prepare_source(input: &ReinstallJourney<'_>, plan: &FleetEnsurePlan) {
    let configured = input
        .desired
        .canisters
        .iter()
        .find(|canister| {
            canister.kind == canic_host::fleet_ensure::model::DesiredCanisterKind::Root
        })
        .unwrap();
    let wasm = configured.wasm.as_deref().unwrap();
    let bytes = std::fs::read(input.adapter_root.join(wasm)).unwrap();
    let principals = input
        .desired
        .canisters
        .iter()
        .map(|canister| (canister.name.clone(), canister.principal.clone().unwrap()))
        .collect();
    let arguments = canic_host::fleet_ensure::ops::compile_arguments(
        &canic_host::fleet_ensure::ops::CanicInitRequest {
            desired: input.desired,
            init: configured.canic_init.as_ref().unwrap(),
            operation_id: &plan.operation_id,
            principals: &principals,
            root: input.adapter_root,
            wasm,
            wasm_sha256: &sha256_hex(&bytes),
        },
    )
    .unwrap();
    let operator = Principal::from_text(&input.desired.operator).unwrap();
    // Supply an installed Root bound to the source operation, then let that real
    // operation install Store and execute its own journaled successor phases.
    input
        .pic
        .reinstall_canister(input.root, bytes, arguments, Some(operator))
        .unwrap();
    prepare_ready_imports(input.pic, input.root, operator, input.pools);
    input
        .pic
        .uninstall_canister(input.store, Some(operator))
        .unwrap();
}

#[expect(
    clippy::too_many_lines,
    reason = "one connected completed-source journey keeps publication, lost install response and terminal replay together"
)]
pub(super) fn assert_journey(input: ReinstallJourney<'_>) {
    let span = Span::start("completed_source_reset_journey");
    let root = input.adapter_root;
    std::fs::create_dir_all(root.join("apps/test")).unwrap();
    std::fs::copy(input.config, root.join("apps/test/canic.toml")).unwrap();
    std::fs::write(root.join("icp.yaml"), "canisters: []\n").unwrap();
    let old_paths = paths(&input);
    let originals = read_state(&old_paths, &input.desired.fleet)
        .unwrap()
        .principals;
    let replacement = selected_reinstall_artifacts(&input);
    retained_contract::inspect_completed_receipts(root, "local", &input.desired.fleet)
        .expect("audit original paid receipts separately from physical inventory");
    let source = retained_contract::inspect_completed_source(root, "local", &input.desired.fleet)
        .expect("audit completed records before any source effect");
    let source_documents = [&old_paths.plan, &old_paths.journal, &old_paths.state]
        .into_iter()
        .map(|path| (path.clone(), std::fs::read(path).unwrap()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(source.inventory.canisters.len(), originals.len());
    retained_contract::check(root, "local", &input.desired.fleet).unwrap();
    let executable = local_icp(&input);
    let icp = canic_host::icp::IcpCli::new(executable.to_str().unwrap(), Some("local".into()))
        .with_cwd(root)
        .with_local_replica(Some(input.local_replica.clone()));
    let planning = cli_receipt(
        root,
        &executable,
        &[
            "fleet",
            "ensure",
            &input.desired.fleet,
            "--reinstall",
            "--json",
        ],
        "completed_preparation",
        None,
        true,
    );
    assert!(
        planning
            .iter()
            .any(|event| event["event"] == "icp_request_timing")
    );
    let preparation = completed_preparation::review(root, "local", &input.desired.fleet)
        .unwrap()
        .unwrap();
    let prepared_receipt = cli_receipt(
        root,
        &executable,
        &[
            "fleet",
            "ensure",
            &input.desired.fleet,
            "--apply",
            &preparation.review_sha256,
            "--json",
        ],
        "completed_preparation",
        Some(&preparation.review_sha256),
        true,
    );
    assert!(
        prepared_receipt
            .iter()
            .any(|event| event["event"] == "completed_preparation_authority"
                && event["data"]["prepared"] == true)
    );
    let unavailable =
        canic_host::icp::IcpCli::new("/missing/completed-reset-icp", Some("local".into()))
            .with_cwd(root);
    let prepared = completed_preparation::apply(
        root,
        "local",
        &input.desired.fleet,
        &preparation.review_sha256,
        &unavailable,
    )
    .expect("prepared replay performs no remote calls");
    assert!(prepared.prepared);
    for (path, bytes) in &source_documents {
        assert_eq!(&std::fs::read(path).unwrap(), bytes);
    }
    let desired = generate(&input, &executable, replacement.release_build_id);
    let digest = desired_sha256(&desired);
    std::fs::write(
        root.join("desired-reset.toml"),
        toml::to_string_pretty(&desired).unwrap(),
    )
    .unwrap();
    let reset_review = cli_receipt(
        root,
        &executable,
        &[
            "fleet",
            "ensure",
            &desired.fleet,
            "--desired",
            "desired-reset.toml",
            "--reinstall",
            "--json",
        ],
        "completed_reset",
        None,
        true,
    );
    assert!(
        reset_review
            .iter()
            .any(|event| event["event"] == "completed_reset_authority")
    );
    let review = completed_reset::review(root, "local", &desired.fleet)
        .unwrap()
        .unwrap();
    assert!(!review.committed);
    let plan = completed_reset::approve(
        root,
        "local",
        &desired.fleet,
        &review.publication.review_sha256,
        &icp,
    )
    .expect("commit fresh authority without issuing installs");
    // Drop all source observations here, as if the host stopped after local publication.
    let resumed = completed_reset::approve(
        root,
        "local",
        &desired.fleet,
        &review.publication.review_sha256,
        &unavailable,
    )
    .expect("committed publication recovery requires no source calls");
    assert_eq!(resumed, plan);
    let failed_receipt = cli_apply_receipt(
        root,
        Path::new("/missing/reset-icp"),
        &desired.fleet,
        &review.publication.review_sha256,
        true,
        false,
    );
    assert_eq!(failed_receipt.last().unwrap()["data"]["state"], "failed");
    let platform = || {
        literal_zero_journey_platform(
            &desired,
            input.icp_wrapper,
            root,
            input.local_replica.clone(),
            true,
        )
    };
    std::fs::write(root.join("lose-install-response"), []).unwrap();
    let first = fleet_ensure_workflow::apply(
        root,
        &desired,
        &digest,
        &desired.fleet,
        &plan.plan_sha256,
        &mut platform(),
    );
    assert!(
        root.join("lost-install-response").exists(),
        "exercise a real lost reinstall response: {first:?}"
    );
    if let Err(error) = &first {
        assert!(
            matches!(error, EnsureWorkflowError::Platform(_)),
            "{error:?}"
        );
    }
    let resumed_receipt = cli_apply_receipt(
        root,
        &executable,
        &desired.fleet,
        &review.publication.review_sha256,
        true,
        true,
    );
    assert!(
        resumed_receipt
            .iter()
            .any(|event| event["event"] == "icp_request_timing")
    );
    assert_eq!(resumed_receipt.last().unwrap()["data"]["terminal"], true);
    let result = fleet_ensure_workflow::apply(
        root,
        &desired,
        &digest,
        &desired.fleet,
        &plan.plan_sha256,
        &mut platform(),
    )
    .expect("recover the installed module and converge the retained estate");
    assert!(result.terminal);
    assert!(result.actual_conservation.is_some());
    assert_eq!(
        read_state(&old_paths, &desired.fleet).unwrap().principals,
        originals
    );
    for bytes in source_documents.values() {
        assert_eq!(
            std::fs::read(
                old_paths
                    .plan
                    .with_file_name("activation-reset-evidence")
                    .join(sha256_hex(bytes))
            )
            .unwrap(),
            *bytes
        );
    }
    let mutations = std::fs::read(root.join("reinstall-mutations.log")).unwrap();
    for json in [false, true] {
        let receipt = cli_apply_receipt(
            root,
            Path::new("/missing/reset-icp"),
            &desired.fleet,
            &review.publication.review_sha256,
            json,
            true,
        );
        assert_eq!(receipt.last().unwrap()["data"]["effects_applied"], 0);
        assert_eq!(
            receipt.last().unwrap()["data"]["plan_sha256"],
            plan.plan_sha256
        );
        assert_eq!(receipt.last().unwrap()["data"]["terminal"], true);
        assert!(
            !receipt
                .iter()
                .any(|event| event["event"] == "icp_request_timing")
        );
        let mut unavailable =
            IcpEnsurePlatform::new(desired.clone(), "/missing/completed-reset-icp", root);
        let replay = fleet_ensure_workflow::apply(
            root,
            &desired,
            &digest,
            &desired.fleet,
            &plan.plan_sha256,
            &mut unavailable,
        )
        .expect("terminal replay is entirely local");
        assert!(replay.terminal);
        assert_eq!(replay.effects_applied, 0);
        assert_eq!(replay.actual_conservation, result.actual_conservation);
    }
    assert_eq!(
        std::fs::read(root.join("reinstall-mutations.log")).unwrap(),
        mutations
    );
    capacity_import::qualify(&input, &desired, &icp);
    span.finish();
}

fn cli_apply_receipt(
    root: &Path,
    executable: &Path,
    fleet: &str,
    approval: &str,
    json: bool,
    succeeds: bool,
) -> Vec<serde_json::Value> {
    let mut args = vec!["fleet", "ensure", fleet, "--apply", approval];
    if json {
        args.push("--json");
    }
    cli_receipt(
        root,
        executable,
        &args,
        "completed_reset",
        Some(approval),
        succeeds,
    )
}

fn cli_receipt(
    root: &Path,
    executable: &Path,
    args: &[&str],
    command: &str,
    approval: Option<&str>,
    succeeds: bool,
) -> Vec<serde_json::Value> {
    let directory = root.join(".canic/diagnostics/fleet");
    std::fs::create_dir_all(&directory).unwrap();
    let before = std::fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<std::collections::BTreeSet<_>>();
    // Run the real CLI in a fresh process. Successful reports stay captured;
    // failed assertions expose the child diagnostics, as ordinary libtest does.
    let mut invocation = vec![
        "--environment",
        "local",
        "--icp",
        executable.to_str().unwrap(),
    ];
    invocation.extend_from_slice(args);
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            CLI_TEST,
            "--exact",
            "--include-ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .current_dir(root)
        .env(CLI_ARGUMENTS, serde_json::to_string(&invocation).unwrap())
        .output()
        .unwrap();
    assert_eq!(
        output.status.success(),
        succeeds,
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("fleet_ensure_timing_receipt") || stderr.contains("Fleet timing receipt:"),
        "{stderr}"
    );
    let paths = std::fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| !before.contains(path))
        .collect::<Vec<_>>();
    assert_eq!(paths.len(), 1, "one receipt per selected invocation");
    let events = std::fs::read_to_string(&paths[0])
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(events[0]["event"], "invocation_started");
    assert_eq!(events[0]["data"]["command"], command);
    assert_eq!(
        events[0]["data"]["applied_review_sha256"],
        serde_json::to_value(approval).unwrap()
    );
    assert!(events[0]["data"]["applied_plan_sha256"].is_null());
    assert_eq!(events.last().unwrap()["event"], "invocation_finished");
    assert_eq!(
        events.last().unwrap()["data"]["timing_evidence_complete"],
        true
    );
    events
}

fn local_icp(input: &ReinstallJourney<'_>) -> PathBuf {
    use std::os::unix::fs::PermissionsExt as _;
    let path = input.adapter_root.join("completed-reset-icp");
    let network = serde_json::json!({"api_url": input.local_replica.url, "root_key": input.local_replica.root_key});
    std::fs::write(
        &path,
        format!(
            r#"#!/bin/bash
set -euo pipefail
case " $* " in
  *" network status "*) printf '%s\n' '{network}'; exit 0 ;;
  *" canister "*|*" cycles "*)
    unset ICP_ENVIRONMENT
    args=()
    while (( $# )); do
      case "$1" in
        -e|--environment|-n|--network|-k|--root-key) shift 2 ;;
        *) args+=("$1"); shift ;;
      esac
    done
    exec '{}' "${{args[@]}}" -n '{}' -k '{}' ;;
  *) exec '{}' "$@" ;;
esac
"#,
            input.icp_wrapper.display(),
            input.local_replica.url,
            input.local_replica.root_key,
            input.icp_wrapper.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path
}

fn generate(
    input: &ReinstallJourney<'_>,
    executable: &Path,
    release: canic_core::ids::ReleaseBuildId,
) -> DesiredFleet {
    let root = input.adapter_root;
    let config = retain_generated_journey_source(root, input.config);
    let seed = root.join("fleet-seed.toml");
    let policy = root.join("fleet-policy.toml");
    let mut seed_value: toml::Value =
        toml::from_str(&std::fs::read_to_string(&seed).unwrap()).unwrap();
    let mut policy_value: toml::Value =
        toml::from_str(&std::fs::read_to_string(&policy).unwrap()).unwrap();
    seed_value["fresh_estate"] = false.into();
    seed_value["coordinator"] = input.coordinator.to_text().into();
    seed_value["roots"][0]["root"] = input.root.to_text().into();
    seed_value["roots"][0]["store"] = input.store.to_text().into();
    let imports = toml::Value::Array(input.pools.iter().map(|id| id.to_text().into()).collect());
    seed_value["roots"][0]["pool_imports"] = imports.clone();
    policy_value["fleet_subnet_roots"][0]["canister_pool"]
        .as_table_mut()
        .unwrap()
        .insert("imports".into(), imports);
    // Keep one spare capacity slot to qualify later current-Fleet enrollment.
    let maximum = policy_value["fleet_subnet_roots"][0]["canister_pool"]["maximum_size"]
        .as_integer()
        .unwrap();
    policy_value["fleet_subnet_roots"][0]["canister_pool"]["maximum_size"] = (maximum + 1).into();
    std::fs::write(&seed, toml::to_string(&seed_value).unwrap()).unwrap();
    std::fs::write(&policy, toml::to_string(&policy_value).unwrap()).unwrap();
    canic_host::fleet_ensure::generate_desired_fleet(
        &canic_host::fleet_ensure::FleetGenerateRequest {
            catalog_progress: None,
            app_config: &config,
            environment: "local",
            fleet: &input.desired.fleet,
            icp_executable: executable.to_str().unwrap(),
            signing_identity: None,
            release_build_id: release,
            root,
            seed: &seed,
            source: &policy,
        },
    )
    .expect("generate fresh current authority without decoding source executable records")
    .desired
}
