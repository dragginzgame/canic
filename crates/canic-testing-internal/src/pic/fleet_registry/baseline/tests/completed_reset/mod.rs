//! Qualify the public clean-reinstall CLI, diagnostic receipts and exact recovery.
//!
//! All remote effects use production adapters against disposable PocketIC canisters.

mod capacity_import;

use super::*;
use canic_core::cdk::utils::hash::sha256_hex;
use canic_host::fleet_ensure::ops::{
    EnsurePaths, capacity_import::journal::CapacityImportJournalStore, read_plan, read_state,
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
    reason = "one public CLI journey retains review, lost replies, receipts and terminal replay"
)]
pub(super) fn assert_journey(input: ReinstallJourney<'_>) {
    let span = Span::start("completed_source_reset_journey");
    let root = input.adapter_root;
    std::fs::create_dir_all(root.join("apps/test")).unwrap();
    std::fs::copy(input.config, root.join("apps/test/canic.toml")).unwrap();
    std::fs::write(root.join("icp.yaml"), "canisters: []\n").unwrap();
    let paths = paths(&input);
    let originals = read_state(&paths, &input.desired.fleet).unwrap().principals;
    let source_documents = [&paths.plan, &paths.journal, &paths.state]
        .into_iter()
        .map(|path| (path.clone(), std::fs::read(path).unwrap()))
        .collect::<BTreeMap<_, _>>();
    let replacement = selected_reinstall_artifacts(&input);
    let executable = local_icp(&input);
    let icp = canic_host::icp::IcpCli::new(executable.to_str().unwrap(), Some("local".into()))
        .with_cwd(root)
        .with_local_replica(Some(input.local_replica.clone()));
    let initial = generate(
        &input,
        &executable,
        input.desired.bootstrap.as_ref().unwrap().release_build_id,
    );
    let desired = generate(&input, &executable, replacement.release_build_id);
    std::fs::write(
        root.join("desired-reset.toml"),
        toml::to_string_pretty(&initial).unwrap(),
    )
    .unwrap();
    let review = || {
        cli_receipt(
            root,
            &executable,
            &[
                "fleet",
                "ensure",
                &desired.fleet,
                "--desired",
                "desired-reset.toml",
                "--source",
                "fleet-policy.toml",
                "--seed",
                "fleet-seed.toml",
                "--reinstall",
                "--json",
            ],
            "ensure",
            None,
            true,
        )
    };
    let operator = Principal::from_text(&desired.operator).unwrap();
    let ledger = Principal::from_text(&desired.cycles_ledger).unwrap();
    let before_operator = ledger_account_balance(input.pic, ledger, operator);
    let planning = review();
    assert!(
        planning
            .iter()
            .any(|event| event["event"] == "icp_request_timing")
    );
    let unpaid = read_plan(&paths).unwrap().unwrap();
    assert_eq!(
        unpaid
            .reviewed_desired
            .as_ref()
            .unwrap()
            .desired()
            .bootstrap
            .as_ref()
            .unwrap()
            .release_build_id,
        initial.bootstrap.as_ref().unwrap().release_build_id
    );
    assert_ne!(
        initial.bootstrap.as_ref().unwrap().release_build_id,
        replacement.release_build_id
    );
    let original_review = std::fs::read(&paths.plan).unwrap();
    let cancelled = cli_output(
        root,
        Path::new("/missing/cancellation-icp"),
        &[
            "fleet",
            "ensure",
            &desired.fleet,
            "--cancel-reinstall",
            &unpaid.plan_sha256,
            "--json",
        ],
        true,
    );
    assert!(String::from_utf8_lossy(&cancelled.stdout).contains("clean_reinstall_cancelled"));
    assert!(!paths.plan.exists() && !paths.journal.exists());
    let archived = root
        .join(".canic/fleet-ensure/history/local")
        .join(&desired.fleet)
        .join("objects")
        .join(sha256_hex(&original_review));
    assert_eq!(std::fs::read(archived).unwrap(), original_review);
    assert_eq!(
        ledger_account_balance(input.pic, ledger, operator),
        before_operator
    );
    let regenerated = generate(&input, &executable, replacement.release_build_id);
    std::fs::write(
        root.join("desired-reset.toml"),
        toml::to_string_pretty(&regenerated).unwrap(),
    )
    .unwrap();
    review();
    let infrastructure = read_plan(&paths).unwrap().unwrap();
    assert_ne!(infrastructure.plan_sha256, unpaid.plan_sha256);
    assert_eq!(
        infrastructure
            .reviewed_desired
            .as_ref()
            .unwrap()
            .desired()
            .bootstrap
            .as_ref()
            .unwrap()
            .release_build_id,
        replacement.release_build_id
    );
    let current_review = std::fs::read(&paths.plan).unwrap();
    cli_output(
        root,
        Path::new("/missing/cancellation-icp"),
        &[
            "fleet",
            "ensure",
            &desired.fleet,
            "--cancel-reinstall",
            &unpaid.plan_sha256,
        ],
        false,
    );
    assert_eq!(std::fs::read(&paths.plan).unwrap(), current_review);
    assert_eq!(
        infrastructure.scope,
        canic_host::fleet_ensure::model::FleetEnsurePlanScope::InfrastructureBootstrap
    );
    // A real install succeeds remotely and loses its reply. The same CLI digest resumes it.
    std::fs::write(root.join("lose-install-response"), []).unwrap();
    let failed = cli_apply_receipt(
        root,
        &executable,
        &desired.fleet,
        &infrastructure.plan_sha256,
        true,
        false,
    );
    assert_eq!(failed.last().unwrap()["data"]["state"], "failed");
    assert!(
        root.join("lost-install-response").is_file(),
        "{}",
        std::fs::read_to_string(root.join("last-cli-output.txt")).unwrap()
    );
    let issued = std::fs::read(&paths.journal).unwrap();
    assert!(matches!(
        canic_host::fleet_ensure::workflow::clean_reinstall::cancel_review(
            root,
            "local",
            &desired.fleet,
            &infrastructure.plan_sha256
        ),
        Err(canic_host::fleet_ensure::ops::EnsureStateError::ResetReviewEffectEvidence { .. })
    ));
    assert_eq!(std::fs::read(&paths.journal).unwrap(), issued);
    let initialized = cli_apply_receipt(
        root,
        &executable,
        &desired.fleet,
        &infrastructure.plan_sha256,
        true,
        true,
    );
    assert_eq!(initialized.last().unwrap()["data"]["terminal"], true);
    assert!(
        initialized.last().unwrap()["data"]["effects_applied"]
            .as_u64()
            .unwrap()
            > 0
    );
    review();
    let import = CapacityImportJournalStore::open(&paths)
        .unwrap()
        .read()
        .unwrap()
        .unwrap();
    let import_digest = canic_core::cdk::utils::hash::hex_bytes(
        import.operation.as_ref().unwrap().review.review_sha256,
    );
    cli_apply_receipt(
        root,
        &executable,
        &desired.fleet,
        &import_digest,
        true,
        true,
    );
    for child in input.pools {
        let status = input.pic.canister_status(*child, Some(input.root)).unwrap();
        assert!(status.module_hash.is_none());
        assert_eq!(status.memory_metrics.stable_memory_size, Nat::from(0_u8));
    }
    review();
    let plan = read_plan(&paths).unwrap().unwrap();
    assert_eq!(
        plan.scope,
        canic_host::fleet_ensure::model::FleetEnsurePlanScope::Full
    );
    let completed = cli_apply_receipt(
        root,
        &executable,
        &desired.fleet,
        &plan.plan_sha256,
        true,
        true,
    );
    assert_eq!(completed.last().unwrap()["data"]["terminal"], true);
    assert_eq!(
        read_state(&paths, &desired.fleet).unwrap().principals,
        originals
    );
    let unavailable = canic_host::icp::IcpCli::new("/missing/reset-icp", Some("local".into()));
    let mut offline = IcpEnsurePlatform::new(desired.clone(), "/missing/reset-icp", root);
    let canic_host::fleet_ensure::workflow::clean_reinstall::CleanReinstallReport::Fleet(result) =
        canic_host::fleet_ensure::workflow::clean_reinstall::apply(
            root,
            "local",
            &desired.fleet,
            &plan.plan_sha256,
            &mut offline,
            &unavailable,
        )
        .expect("completed reset replays without an IC executable")
    else {
        panic!("expected completed Fleet")
    };
    let actual = result.actual_conservation.as_ref().unwrap();
    assert_eq!(
        ledger_account_balance(input.pic, ledger, operator),
        before_operator
            - Nat::from(
                infrastructure.conservation.maximum_operator_debit_cycles
                    + actual.operator_debit_cycles
            )
    );
    let history = root
        .join(".canic/fleet-ensure/history/local")
        .join(&desired.fleet)
        .join("objects");
    for bytes in source_documents.values() {
        assert_eq!(
            std::fs::read(history.join(sha256_hex(bytes))).unwrap(),
            *bytes
        );
    }
    let mutations = std::fs::read(root.join("reinstall-mutations.log")).unwrap();
    // Earlier phase receipts remain local replay after the active plan advances.
    let replay = cli_apply_receipt(
        root,
        Path::new("/missing/reset-icp"),
        &desired.fleet,
        &infrastructure.plan_sha256,
        true,
        true,
    );
    assert_eq!(
        replay[0]["data"]["applied_plan_sha256"],
        infrastructure.plan_sha256
    );
    assert_eq!(replay.last().unwrap()["data"]["effects_applied"], 0);
    assert!(
        !replay
            .iter()
            .any(|event| event["event"] == "icp_request_timing")
    );
    for json in [false, true] {
        let receipt = cli_apply_receipt(
            root,
            Path::new("/missing/reset-icp"),
            &desired.fleet,
            &plan.plan_sha256,
            json,
            true,
        );
        let outcome = &receipt.last().unwrap()["data"];
        assert_eq!(outcome["effects_applied"], 0);
        assert_eq!(outcome["plan_sha256"], plan.plan_sha256);
        assert_eq!(outcome["terminal"], true);
        assert!(
            !receipt
                .iter()
                .any(|event| event["event"] == "icp_request_timing")
        );
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
    cli_receipt(root, executable, &args, "ensure", Some(approval), succeeds)
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
    let output = cli_output(root, executable, args, succeeds);
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
    let invocation = &events[0]["data"];
    assert_eq!(
        invocation["applied_plan_sha256"]
            .as_str()
            .or_else(|| invocation["applied_review_sha256"].as_str()),
        approval
    );
    assert!(
        invocation["applied_plan_sha256"].is_null()
            || invocation["applied_review_sha256"].is_null()
    );
    if succeeds {
        assert!(
            events
                .iter()
                .any(|event| event["event"] == "clean_reinstall_authority")
        );
    }
    assert_eq!(events.last().unwrap()["event"], "invocation_finished");
    assert_eq!(
        events.last().unwrap()["data"]["timing_evidence_complete"],
        true
    );
    events
}

fn cli_output(
    root: &Path,
    executable: &Path,
    args: &[&str],
    succeeds: bool,
) -> std::process::Output {
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
    std::fs::write(
        root.join("last-cli-output.txt"),
        format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
    .unwrap();
    assert_eq!(
        output.status.success(),
        succeeds,
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

pub(super) fn local_icp(input: &ReinstallJourney<'_>) -> PathBuf {
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

pub(super) fn generate(
    input: &ReinstallJourney<'_>,
    executable: &Path,
    release: canic_core::ids::ReleaseBuildId,
) -> DesiredFleet {
    let root = input.adapter_root;
    let config = retain_generated_journey_source(root, input.config);
    let seed = root.join("fleet-seed.toml");
    let policy = root.join("fleet-policy.toml");
    let preflight = canic_host::fleet_ensure::FleetGenerationInputsRequest {
        root,
        environment: "local",
        fleet: &input.desired.fleet,
        source: &policy,
        seed: &seed,
    };
    let operator = Principal::from_text(&input.desired.operator).unwrap();
    let ledger = Principal::from_text(&input.desired.cycles_ledger).unwrap();
    let mut seed_value: toml::Value =
        toml::from_str(&std::fs::read_to_string(&seed).unwrap()).unwrap();
    let mut policy_value: toml::Value =
        toml::from_str(&std::fs::read_to_string(&policy).unwrap()).unwrap();
    if seed_value
        .get("fresh_estate")
        .and_then(toml::Value::as_bool)
        == Some(true)
    {
        assert!(matches!(
            canic_host::fleet_ensure::validate_generation_inputs(&preflight, operator, ledger),
            Err(canic_host::fleet_ensure::FleetGenerateError::CompletedFleetRequiresExplicitInventory)
        ));
    }
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
    canic_host::fleet_ensure::validate_generation_inputs(&preflight, operator, ledger)
        .expect("explicit current inventory passes before artifact-bound generation");
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
