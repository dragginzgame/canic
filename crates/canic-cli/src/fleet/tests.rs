use super::*;
use crate::test_support::temp_dir;
use canic_host::fleet_ensure::dto::FleetEnsurePhase;
use canic_host::fleet_ensure::{
    model::{
        ActualCycleConservation, CanisterDisposition, CanisterPlan, CanisterRuntimeStatus,
        CycleConservation, DesiredFleet, DesiredFleetArtifacts, EnsureAction,
        EstateFundingDomainPlan, FleetEnsureCompletion, FleetEnsureJournalRecord, FleetEnsurePlan,
        FleetEnsurePlanScope, FleetObservation, FundingPauseRecord, FundingReviewRecord,
        LiveCanister, NativeFundingRequiredRecord,
    },
    ops::{EnsurePaths, write_journal, write_plan},
    policy::compile_plan,
};
use std::collections::BTreeMap;

#[test]
fn fleet_commands_are_current_generation_and_lexicographically_ordered() {
    let command = fleet_command();
    let names = command
        .get_subcommands()
        .map(clap::Command::get_name)
        .collect::<Vec<_>>();
    assert_eq!(names, ["ensure", "generate", "readiness"]);
}

#[test]
fn fleet_identity_is_explicit_for_generation_readiness_and_all_ensure_paths() {
    let release = "01".repeat(32);
    let generation = GenerateOptions::parse(
        [
            "generate",
            "staging",
            "--app-config",
            "canic.toml",
            "--release-build",
            &release,
            "--identity",
            "operator-a",
        ]
        .into_iter()
        .map(OsString::from),
    )
    .unwrap();
    assert_eq!(generation.identity.as_deref(), Some("operator-a"));
    for extra in [
        vec![],
        vec!["--operator-mint"],
        vec!["--observe-funding", "root"],
    ] {
        let mut args = vec!["ensure", "staging", "--identity", "operator-b"];
        args.extend(extra);
        let options = EnsureOptions::parse(args.into_iter().map(OsString::from)).unwrap();
        assert_eq!(options.identity.as_deref(), Some("operator-b"));
    }
    let readiness = parse_matches(
        readiness::command(),
        [
            "staging",
            "--operator",
            "aaaaa-aa",
            "--identity",
            "operator-c",
        ]
        .into_iter()
        .map(OsString::from),
    )
    .unwrap();
    assert_eq!(
        string_option(&readiness, "identity").as_deref(),
        Some("operator-c")
    );
    assert!(matches!(
        EnsureOptions::parse(
            ["ensure", "staging", "--identity", ""]
                .into_iter()
                .map(OsString::from)
        ),
        Err(FleetCommandError::Usage(_))
    ));
}

#[test]
fn operator_mint_options_separate_review_payment_and_cancellation() {
    let parse = |args: &[&str]| EnsureOptions::parse(args.iter().map(OsString::from));
    let review = parse(&["ensure", "staging", "--operator-mint"]).unwrap();
    assert!(review.operator_mint);
    assert!(review.apply.is_none());
    assert!(review.cancel_mint.is_none());
    let digest = "31".repeat(32);
    let approved = parse(&["ensure", "staging", "--operator-mint", "--apply", &digest]).unwrap();
    assert_eq!(approved.apply.as_deref(), Some(digest.as_str()));
    let cancelled = parse(&[
        "ensure",
        "staging",
        "--operator-mint",
        "--cancel-mint",
        &digest,
    ])
    .unwrap();
    assert_eq!(cancelled.cancel_mint.as_deref(), Some(digest.as_str()));
    for args in [
        vec!["ensure", "staging", "--operator-mint", "--reinstall"],
        vec!["ensure", "staging", "--cancel-mint", &digest],
        vec![
            "ensure",
            "staging",
            "--operator-mint",
            "--cancel-mint",
            &digest,
            "--apply",
            &digest,
        ],
        vec![
            "ensure",
            "staging",
            "--mint-cmc",
            "rkp4c-7iaaa-aaaaa-aaaca-cai",
        ],
    ] {
        assert!(matches!(parse(&args), Err(FleetCommandError::Usage(_))));
    }
    assert!(!parse(&["ensure", "staging"]).unwrap().operator_mint);
}

#[test]
fn generate_defaults_to_policy_seed_and_desired_paths() {
    let release = "01".repeat(32);
    let options = GenerateOptions::parse([
        OsString::from("generate"),
        OsString::from("staging"),
        OsString::from("--app-config"),
        OsString::from("apps/demo/canic.toml"),
        OsString::from("--release-build"),
        OsString::from(release),
    ])
    .expect("parse generation");

    assert_eq!(options.source, PathBuf::from("deployments/staging.toml"));
    assert_eq!(
        options.seed,
        PathBuf::from("deployments/staging.estate.toml")
    );
    assert_eq!(options.output, PathBuf::from("fleets/staging.toml"));
    assert_eq!(options.replace, None);
    assert!(!options.fresh);
    assert_eq!(options.management_creation_fee_cycles, None);
}

#[test]
fn fresh_generation_requires_and_retains_exact_creation_fee_authority() {
    let release = "01".repeat(32);
    let args = [
        OsString::from("generate"),
        OsString::from("staging"),
        OsString::from("--app-config"),
        OsString::from("apps/demo/canic.toml"),
        OsString::from("--release-build"),
        OsString::from(&release),
        OsString::from("--fresh"),
    ];
    assert!(matches!(
        GenerateOptions::parse(args.clone()),
        Err(FleetCommandError::Usage(_))
    ));
    let options = GenerateOptions::parse(args.clone().into_iter().chain([
        OsString::from("--management-creation-fee-cycles"),
        OsString::from("500B"),
    ]))
    .expect("parse fresh generation");
    assert!(options.fresh);
    assert_eq!(
        options.management_creation_fee_cycles,
        Some(500_000_000_000)
    );
    assert_eq!(options.cycles_ledger, DEFAULT_CYCLES_LEDGER);

    for invalid in ["500000000000", "500b", "0.5e3B"] {
        let error = GenerateOptions::parse(args.clone().into_iter().chain([
            OsString::from("--management-creation-fee-cycles"),
            OsString::from(invalid),
        ]))
        .expect_err("reject non-canonical human cycle input");
        assert!(matches!(error, FleetCommandError::Usage(_)));
    }
}

#[test]
fn ensure_defaults_to_current_fleet_document() {
    let options = EnsureOptions::parse([OsString::from("ensure"), OsString::from("staging")])
        .expect("parse ensure");
    assert_eq!(options.desired, PathBuf::from("fleets/staging.toml"));
    assert_eq!(options.apply, None);
}

#[test]
fn ensure_requires_canonical_apply_digest() {
    let error = EnsureOptions::parse([
        OsString::from("ensure"),
        OsString::from("staging"),
        OsString::from("--apply"),
        OsString::from("not-a-digest"),
    ])
    .expect_err("reject invalid digest");
    assert!(matches!(error, FleetCommandError::Usage(_)));
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one retained fixture compares ordinary resume with explicit reset input selection"
)]
fn ensure_reopens_retained_reviewed_input_when_working_toml_is_missing() {
    let root = temp_dir("canic-cli-retained-desired");
    let principal = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    let controller = "rdmx6-jaaaa-aaaaa-aaadq-cai";
    let desired = toml::from_str::<DesiredFleet>(
        r#"
cycles_ledger = "um5iw-rqaaa-aaaaq-qaaba-cai"
environment = "local"
fleet = "retained"
ledger_fee_cycles = "0"
management_creation_fee_cycles = "0"
material_cycle_threshold = "1"
maximum_observation_burn_cycles = "1"
maximum_stalled_observations = 2
maximum_update_burn_cycles = "1"
operator = "rdmx6-jaaaa-aaaaa-aaadq-cai"
schema_version = 1
treasury = "coordinator"

[[canisters]]
controllers = ["rdmx6-jaaaa-aaaaa-aaadq-cai"]
initial_cycles = "20"
kind = "coordinator"
minimum_cycles = "20"
name = "coordinator"
presence = "present"
principal = "rrkah-fqaaa-aaaaa-aaaaq-cai"
replace = false
subnet = "rwlgt-iiaaa-aaaaa-aaaaa-cai"
"#,
    )
    .expect("parse current desired fixture");
    let desired_sha256 = "35".repeat(32);
    let mut plan = compile_plan(
        &desired,
        &DesiredFleetArtifacts::default(),
        &[],
        &desired_sha256,
        "retained",
        &FleetObservation {
            additional_controlled_cycles: BTreeMap::new(),
            canisters: BTreeMap::from([(
                "coordinator".to_string(),
                Some(LiveCanister {
                    canister_version: Some(1),
                    controllers: vec![controller.to_string()],
                    cycles: 20,
                    module_sha256: None,
                    principal: principal.to_string(),
                    reinstall_required: false,
                    root_owned_lifecycle: None,
                    status: CanisterRuntimeStatus::Running,
                }),
            )]),
            estate_funding_domains: BTreeMap::new(),
            ledger_fee_cycles: 0,
            operator_cycles: 0,
            protocol_ready: BTreeMap::new(),
        },
        1,
        &"36".repeat(32),
        None,
    )
    .expect("compile retained desired authority");
    let paths = EnsurePaths::under(&root, "local", "retained");
    write_plan(&paths, &plan).expect("retain reviewed plan");
    write_journal(
        &paths,
        &FleetEnsureJournalRecord {
            funding_observations: BTreeMap::new(),
            funding_reviews: Vec::new(),
            successor_phases: Vec::new(),
            completion: FleetEnsureCompletion::InProgress,
            estate_funding_required: None,
            effects: Vec::new(),
            fleet: "retained".to_string(),
            initial_controlled_cycles: 20,
            initial_estate_funding_cycles_by_root: BTreeMap::new(),
            initial_operator_cycles: 0,
            operation_id: plan.operation_id.clone(),
            plan_sha256: plan.plan_sha256.clone(),
            schema_version: 1,
            stalled_observations: 0,
        },
    )
    .expect("retain in-progress journal");
    let mut options = EnsureOptions {
        observe_funding: None,
        operator_mint: false,
        mint_cmc: "rkp4c-7iaaa-aaaaa-aaaca-cai".into(),
        mint_icp_ledger: "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
        cancel_mint: None,
        reinstall: false,
        apply: Some(plan.plan_sha256.clone()),
        desired: PathBuf::from("missing.toml"),
        environment: Some("local".to_string()),
        fleet: "retained".to_string(),
        icp: "icp".to_string(),
        identity: None,
        json: false,
    };

    let loaded = load_ensure_authority(&root, &root.join("missing.toml"), &options)
        .expect("load exact retained desired without working TOML");
    assert_eq!(loaded.desired, desired);
    assert_eq!(loaded.sha256, desired_sha256);

    // An exact completed wipe apply also recovers its selected input after a lost final acknowledgement.
    plan.reinstall = Some(Box::new(
        canic_host::fleet_ensure::model::FleetReinstallRecord {
            activation_reset: None,
            source: Some(Box::new(
                canic_host::fleet_ensure::model::FleetReinstallSourceRecord {
                    terminal_retirement: None,
                    reviewed_desired: *plan.reviewed_desired.clone().unwrap(),
                    wasm_sha256_by_canister: BTreeMap::new(),
                    candid_sha256_by_path: BTreeMap::new(),
                },
            )),
            target_artifacts_sha256: Some("51".repeat(32)),
            operation_id: plan.operation_id.clone(),
            source_operation_id: "52".repeat(32),
            authorities: Vec::new(),
            assets: Vec::new(),
        },
    ));
    plan.plan_sha256 = canic_host::fleet_ensure::policy::expected_plan_sha256(&plan);
    write_plan(&paths, &plan).unwrap();
    let mut journal = canic_host::fleet_ensure::ops::read_journal(&paths)
        .unwrap()
        .unwrap();
    journal.completion = FleetEnsureCompletion::Converged;
    journal.plan_sha256.clone_from(&plan.plan_sha256);
    write_journal(&paths, &journal).unwrap();
    options.apply = Some(plan.plan_sha256.clone());
    let replay = load_ensure_authority(&root, &root.join("missing.toml"), &options).unwrap();
    assert_eq!(replay.desired, desired);
    assert_eq!(replay.sha256, desired_sha256);
    options.apply = Some("53".repeat(32));
    assert!(load_ensure_authority(&root, &root.join("missing.toml"), &options).is_err());

    let mut reset_options = options;
    reset_options.reinstall = true;
    reset_options.apply = None;
    let current_path = root.join("current.toml");
    fs::write(
        &current_path,
        toml::to_string(&desired).expect("current desired"),
    )
    .expect("write current desired");
    // Deliberate reset diagnoses the source independently; its old plan need not
    // deserialize as an executable current plan merely to load today's request.
    fs::write(&paths.plan, br#"{"source_evidence_only":true}"#).expect("opaque source plan");
    let loaded = load_ensure_authority(&root, &current_path, &reset_options)
        .expect("load the explicit reset request independently of source execution");
    assert_eq!(loaded.desired, desired);

    fs::remove_dir_all(root).expect("remove test directory");
}

#[test]
fn generated_desired_output_requires_exact_digest_for_replacement() {
    let root = temp_dir("canic-fleet-generated-output");
    let path = root.join("fleets/staging.toml");

    publish_generated(&path, b"first", None).expect("create generated output");
    publish_generated(&path, b"first", None).expect("repeat exact output");
    assert!(matches!(
        publish_generated(&path, b"second", None),
        Err(FleetCommandError::OutputConflict(conflict)) if conflict == path
    ));
    assert!(matches!(
        publish_generated(&path, b"second", Some(&"00".repeat(32))),
        Err(FleetCommandError::OutputDigestMismatch { path: conflict, .. }) if conflict == path
    ));
    publish_generated(&path, b"second", Some(&sha256_hex(b"first")))
        .expect("replace exact reviewed output");
    assert_eq!(fs::read(&path).expect("read replaced output"), b"second");

    let missing = root.join("fleets/missing.toml");
    assert!(matches!(
        publish_generated(&missing, b"first", Some(&sha256_hex(b"absent"))),
        Err(FleetCommandError::OutputMissingForReplacement(path)) if path == missing
    ));

    fs::remove_dir_all(root).expect("remove test directory");
}

#[test]
fn generate_replace_requires_canonical_digest() {
    let release = "01".repeat(32);
    let error = GenerateOptions::parse([
        OsString::from("generate"),
        OsString::from("staging"),
        OsString::from("--app-config"),
        OsString::from("apps/demo/canic.toml"),
        OsString::from("--release-build"),
        OsString::from(release),
        OsString::from("--replace"),
        OsString::from("not-a-digest"),
    ])
    .expect_err("reject invalid replacement digest");
    assert!(matches!(error, FleetCommandError::Usage(_)));
}

fn cycle_quantity_report(principal: &str) -> FleetEnsureReport {
    FleetEnsureReport {
        funding_review: None,
        actual_conservation: Some(ActualCycleConservation {
            estate_funding_cycles: 10_000_000_000_000,
            exact_estate_creation_fee_cycles: 500_000_000_000,
            exact_unavoidable_fee_cycles: 3_500_700_000_000,
            final_controlled_cycles: 1_001_498_000_000_000,
            observed_net_cycle_debit_cycles: 2_000_000_000,
            observed_starting_cycles: 1_000_000_000_000_000,
            observed_net_cycle_credit_cycles: 0,
            operator_debit_cycles: 1_500_000_000_000,
            received_new_funding_cycles: 1_500_000_000_000,
        }),
        effects_applied: 1,
        plan: FleetEnsurePlan {
            continuation: None,
            canisters: vec![CanisterPlan {
                actions: vec![EnsureAction::Fund {
                    pool_funding: None,
                    amount: 1_000_000_000_000_000,
                    created_at_time: 1,
                    expected_post_cycles: 1_002_000_000_000,
                    funding_deficit_cycles: 2_000_000_000,
                    funding_margin_cycles: 500_000_000,
                    ledger: "ledger".to_string(),
                    name: "app".to_string(),
                    principal: principal.to_string(),
                }],
                disposition: CanisterDisposition::Reuse,
                name: "app".to_string(),
                observed_cycles: 1_250_000_000,
                principal: Some(principal.to_string()),
            }],
            conservation: CycleConservation {
                estate_funding_domains: vec![EstateFundingDomainPlan {
                    allocated_workloads: 0,
                    available_cycles: Some(4_000_000_000_000),
                    available_pool_slots: 2,
                    creation_amount_cycles: 6_500_000_000_000,
                    creation_execution_margin_cycles: 1_000_000_000_000,
                    readiness_floor_cycles: 5_000_000_000_000,
                    cycles_ledger: "estate-ledger".to_string(),
                    eligible_ready_pool_assets: 0,
                    initial_pool_assets: Vec::new(),
                    ledger_fee_cycles: 100_000_000,
                    management_creation_fee_cycles: 500_000_000_000,
                    maximum_creation_debit_cycles: 13_000_200_000_000,
                    maximum_creation_fee_cycles: 1_000_200_000_000,
                    maximum_funding_cycles: 9_000_200_000_000,
                    occupied_pool_assets: 0,
                    pending_creation_count: 0,
                    pending_creation: None,
                    planned_initial_workloads: 2,
                    pool_maximum_size: 2,
                    required_creation_count: 2,
                    root: "root-0".to_string(),
                    root_principal: Some(principal.to_string()),
                    shortfall_cycles: 9_000_200_000_000,
                }],
                expected_post_operation_cycles: 101_600_000_000_000,
                maximum_execution_burn_cycles: 82_000_000_000_000,
                maximum_new_funding_cycles: 175_600_000_000_000,
                maximum_operator_debit_cycles: 179_100_700_000_000,
                maximum_unavoidable_fee_cycles: 3_500_700_000_000,
                observed_controlled_cycles: 0,
                retained_in_reused_canisters_cycles: 0,
                scheduled_transfer_cycles: 0,
            },
            desired_sha256: "desired".to_string(),
            environment: "local".to_string(),
            fleet: "demo".to_string(),
            operation_id: "operation".to_string(),
            plan_sha256: "plan".to_string(),
            planned_at_time: 1,
            protocol_actions: Vec::new(),
            recovery_review: None,
            reinstall: None,
            root_reinstall_bindings: Vec::new(),
            root_start_authority: None,
            reviewed_desired: None,
            schema_version: 1,
            scope: FleetEnsurePlanScope::Full,
            terminal_inventory_operation_id: None,
        },
        terminal: false,
    }
}

#[test]
fn text_report_formats_every_cycle_quantity_with_three_decimal_units() {
    let principal = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    let report = cycle_quantity_report(principal);
    assert_eq!(
        render_text_report(&report),
        format!(
            "fleet: demo\
             \noperation_id: operation\
             \nplan_sha256: plan\
             \nplan_scope: full\
             \nterminal: false\
             \ncycle_budget: planning allowances; maximums are not measured expenditure\
             \nobserved_controlled_cycles: 0.000B\
             \nretained_in_reused_canisters_cycles: 0.000B\
             \nscheduled_transfer_cycles: 0.000B\
             \nmaximum_unavoidable_fee_cycles: 3.501T\
             \nmaximum_execution_burn_cycles: 82.000T\
             \nmaximum_new_funding_cycles: 175.600T\
             \nmaximum_operator_debit_cycles: 179.101T\
             \nmaximum_estate_funding_cycles: 9.000T\
             \nmaximum_estate_creation_fee_cycles: 1.000T\
             \nexpected_post_operation_cycles: 101.600T\
             \nestate_funding_domains:\
             \n  root-0: root_principal={principal} ledger=estate-ledger balance=4.000T workloads=0/2 pool=0/2 ready=0 pending=0 pending_detail=none available_slots=2 root_funded_creations=2 creation_amount=6.500T readiness_floor=5.000T management_creation_fee=500.000B execution_margin=1.000T ledger_fee=0.100B maximum_debit=13.000T funding=9.000T shortfall=9.000T\
             \nhost_create_actions: 0 (initial or replacement canisters; separate from Root-funded pool creations)\
             \ncanisters:\
             \n  app: disposition=Reuse principal={principal} observed_cycles=1.250B effects=1 actions=[native_topup]\
             \n  native_topup app: cycles_ledger_withdraw=1.000Q ledger=ledger target={principal} deficit=2.000B margin=0.500B expected_native_post=1.002T\
             \nconservation_equation: 0.000B observed controlled + 179.101T maximum operator debit - 3.501T maximum unavoidable fees - 1.000T maximum Root-funded creation fees - 82.000T maximum execution burn = 101.600T expected remaining\
             \nmeasured_estate_funding_cycles: 10.000T\
             \nobserved_conservation: 1.000Q observed starting + 1.500T received funding + 0.000B observed net surplus - 500.000B exact Root-funded creation fees - 2.000B observed net deficit = 1.001Q final controlled"
        )
    );
}

#[test]
fn text_report_distinguishes_host_creates_and_ordered_reinstall_actions() {
    let principal = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    let mut report = cycle_quantity_report(principal);
    report.actual_conservation = None;
    report.plan.conservation.estate_funding_domains[0].required_creation_count = 0;
    report.plan.canisters[0].actions = vec![
        EnsureAction::Create {
            controller_canisters: Vec::new(),
            controllers: Vec::new(),
            created_at_time: 1,
            ledger: "ledger".to_string(),
            name: "pool-0".to_string(),
            requested_initial_cycles: 5 * TC,
            subnet: "subnet".to_string(),
        },
        EnsureAction::Stop {
            name: "app".to_string(),
            principal: principal.to_string(),
        },
        EnsureAction::Install {
            canic_init: None,
            reinstall_witness: None,
            init_arg: None,
            init_arg_sha256: None,
            init_candid: None,
            init_candid_sha256: None,
            mode: InstallMode::Reinstall,
            name: "app".to_string(),
            principal: principal.to_string(),
            wasm: "app.wasm".to_string(),
            wasm_sha256: "00".repeat(32),
        },
        EnsureAction::SetControllers {
            controller_canisters: Vec::new(),
            controllers: Vec::new(),
            name: "app".to_string(),
            principal: principal.to_string(),
        },
        EnsureAction::Start {
            name: "app".to_string(),
            principal: principal.to_string(),
        },
    ];
    let text = render_text_report(&report);
    assert!(text.contains("host_create_actions: 1"));
    assert!(text.contains("root_funded_creations=0"));
    assert!(text.contains("actions=[create, stop, reinstall, set_controllers, start]"));
    assert!(!text.contains("observed_conservation:"));
    let json = report_json_value(&report).expect("structured report");
    assert_eq!(
        json["plan"]["canisters"][0]["actions"][0]["requested_initial_cycles"],
        "5000000000000"
    );
    assert_eq!(
        json["plan"]["canisters"][0]["actions"][2]["mode"],
        "reinstall"
    );
    report.plan.canisters[0].actions.clear();
    let text = render_text_report(&report);
    assert!(text.contains("host_create_actions: 0"));
    assert!(text.contains("effects=0 actions=[]"));
}

#[test]
fn text_report_formats_pending_funding_and_unobserved_balances() {
    let mut report = cycle_quantity_report("rrkah-fqaaa-aaaaa-aaaaq-cai");
    let domain = &mut report.plan.conservation.estate_funding_domains[0];
    domain.available_cycles = None;
    domain.pending_creation = Some(
        canic_host::fleet_ensure::model::EstatePoolPendingCreationObservation {
            attempt_count: 2,
            available_cycles: Some(4 * TC),
            creation_amount_cycles: 6 * TC,
            created_principal: None,
            diagnostic: None,
            last_attempt_at_ns: None,
            operation_id: "pending".to_string(),
            required_cycles: Some(6 * TC),
            retry_at_ns: None,
            shortfall_cycles: Some(2 * TC),
            uncertain_result: false,
        },
    );
    let text = render_text_report(&report);
    assert!(text.contains("balance=unobserved"));
    assert!(text.contains("available:4.000T required:6.000T shortfall:2.000T"));
    report.plan.conservation.estate_funding_domains[0]
        .pending_creation
        .as_mut()
        .expect("pending")
        .available_cycles = None;
    assert!(render_text_report(&report).contains("available:unobserved"));
}

#[test]
fn advancing_progress_refreshes_counts_in_text_and_json() {
    for applied_effects in [0, 1, 16, 28, 34] {
        let progress = FleetEnsureProgress {
            next_action: None,
            operation_id: "e1".repeat(32),
            plan_sha256: "e2".repeat(32),
            phase: FleetEnsurePhase::Infrastructure,
            state: FleetEnsureProgressState::Advancing,
            applied_effects,
            reviewed_effects: 34,
        };
        assert!(
            render_progress(&progress, false)
                .contains(&format!("reviewed effects {applied_effects}/34"))
        );
        let json: serde_json::Value =
            serde_json::from_str(&render_progress(&progress, true)).unwrap();
        assert_eq!(json["progress"]["applied_effects"], applied_effects);
        assert_eq!(json["progress"]["reviewed_effects"], 34);
        assert_eq!(json["progress"]["state"]["kind"], "advancing");
        assert_eq!(json["progress"]["operation_id"], progress.operation_id);
        assert_eq!(json["progress"]["plan_sha256"], progress.plan_sha256);
    }
}

#[test]
fn provisioning_wait_exposes_typed_stage_counts_and_invocation_elapsed() {
    let mut progress = FleetEnsureProgress {
        next_action: None,
        operation_id: "reviewed-operation".into(),
        plan_sha256: "reviewed-plan".into(),
        phase: FleetEnsurePhase::WorkloadProvisioning,
        state: FleetEnsureProgressState::AwaitingProgress {
            elapsed_seconds: 63,
            provisioning: Some(canic_host::fleet_ensure::dto::FleetProvisioningProgress {
                pending_root_failure: None,
                phase: canic_core::dto::component_provisioning::FleetComponentProvisioningPhase::ActivatingRuntimes,
                root_batch_count: 1,
                accepted_root_count: 1,
                provisioned_root_count: 1,
                directory_confirmed_root_count: 1,
                directory_confirmation_root_count: 1,
                runtime_activated_root_count: 0,
                component_count: 3,
            }),
        },
        applied_effects: 50,
        reviewed_effects: 52,
    };
    let text = render_progress(&progress, false);
    for detail in [
        "50/52",
        "63s awaiting this effect here",
        "Waiting for application services to start",
        "prepared Roots 1/1",
        "registered Roots 1/1",
        "active Roots 0/1",
        "Components in scope 3",
    ] {
        assert!(text.contains(detail), "missing {detail} from {text}");
    }
    let value: serde_json::Value = serde_json::from_str(&render_progress(&progress, true)).unwrap();
    assert_eq!(
        value["progress"]["state"],
        serde_json::json!({
            "kind": "awaiting_progress",
            "elapsed_seconds": 63,
            "provisioning": {
                "pending_root_failure": null,
                "phase": "ActivatingRuntimes",
                "root_batch_count": 1,
                "accepted_root_count": 1,
                "provisioned_root_count": 1,
                "directory_confirmed_root_count": 1,
                "directory_confirmation_root_count": 1,
                "runtime_activated_root_count": 0,
                "component_count": 3,
            },
        })
    );
    progress.state = FleetEnsureProgressState::AwaitingProgress {
        elapsed_seconds: 0,
        provisioning: None,
    };
    let value: serde_json::Value = serde_json::from_str(&render_progress(&progress, true)).unwrap();
    assert_eq!(
        value["progress"]["state"]["provisioning"],
        serde_json::Value::Null
    );
    assert!(!render_progress(&progress, false).contains("runtime Roots"));
}

#[test]
fn phase_progress_json_has_exact_operation_authority_and_numeric_counts() {
    let progress = FleetEnsureProgress {
        next_action: None,
        operation_id: "e1".repeat(32),
        plan_sha256: "e2".repeat(32),
        phase: FleetEnsurePhase::ImportReconciliation,
        state: FleetEnsureProgressState::PrerequisiteComplete,
        applied_effects: 17,
        reviewed_effects: 22,
    };
    let value: serde_json::Value = serde_json::from_str(&render_progress(&progress, true))
        .expect("one independently parseable progress event");
    assert_eq!(value["event"], "fleet_ensure_progress");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["progress"]["phase"], "import_reconciliation");
    assert_eq!(value["progress"]["operation_id"], progress.operation_id);
    assert_eq!(value["progress"]["plan_sha256"], progress.plan_sha256);
    assert_eq!(value["progress"]["state"]["kind"], "prerequisite_complete");
    assert_eq!(value["progress"]["applied_effects"], 17);
    assert_eq!(value["progress"]["reviewed_effects"], 22);
}

#[test]
fn deliberate_reinstall_cannot_replace_a_reviewed_apply_digest() {
    let args = [
        OsString::from("ensure"),
        OsString::from("staging"),
        OsString::from("--reinstall"),
    ];
    assert!(EnsureOptions::parse(args.clone()).unwrap().reinstall);
    let conflicting = args
        .into_iter()
        .chain([OsString::from("--apply"), OsString::from("11".repeat(32))]);
    assert!(matches!(
        EnsureOptions::parse(conflicting),
        Err(FleetCommandError::Usage(_))
    ));
}

#[test]
fn recovery_review_is_visible_in_text_json_and_typed_progress() {
    use canic_host::fleet_ensure::model::{
        FleetEnsureSuccessorReviewReason, FleetReviewAction, FleetSuccessorReview,
    };
    let report = recovery_quantity_report();
    let value = report_json_value(&report).unwrap();
    assert_eq!(
        value["plan"]["recovery_review"]["continuation_reserve_cycles"],
        "60"
    );
    assert_eq!(
        value["plan"]["recovery_review"]["discovery"],
        "pending_current_protocol"
    );
    assert!(render_text_report(&report).contains("base_execution_burn_cycles:"));
    assert!(render_text_report(&report).contains("continuation_reserve_cycles:"));
    assert_eq!(
        value["plan"]["recovery_review"]["per_step_burn_cycles"],
        "12"
    );
    assert_eq!(
        value["plan"]["recovery_review"]["maximum_successor_actions"],
        8
    );
    let forecast = &value["plan"]["recovery_review"]["startup_funding"][0];
    assert_eq!(forecast["required_native_cycles"], "88");
    assert_eq!(
        forecast["reuse_assumption"],
        "fresh_children_and_full_publication"
    );
    assert!(forecast["unfunded_role"].is_null());
    let rendered = render_text_report(&report);
    assert!(rendered.contains("startup_forecast: Root root;"));
    assert!(rendered.contains("4 steps at"));
    assert!(rendered.contains("requires fresh funding review"));
    let progress = FleetEnsureProgress {
        next_action: None,
        operation_id: "operation".into(),
        plan_sha256: "plan".into(),
        phase: FleetEnsurePhase::TerminalVerification,
        state: FleetEnsureProgressState::ReviewRequired {
            reason: FleetEnsureSuccessorReviewReason::AdditionalEffect,
            review: Some(Box::new(FleetSuccessorReview {
                actions: vec![FleetReviewAction {
                    name: "pool-1".into(),
                    principal: Some("asset".into()),
                    kind: "fund".into(),
                }],
                maximum_additional_debit_cycles: 47,
                next_review_command: "canic fleet ensure 'demo' --desired 'custom.toml'".into(),
            })),
        },
        applied_effects: 2,
        reviewed_effects: 2,
    };
    let value: serde_json::Value = serde_json::from_str(&render_progress(&progress, true)).unwrap();
    assert_eq!(
        value["progress"]["state"]["review"]["actions"][0]["kind"],
        "fund"
    );
    assert_eq!(
        value["progress"]["state"]["review"]["maximum_additional_debit_cycles"],
        "47"
    );
    assert!(render_progress(&progress, false).contains("--desired 'custom.toml'"));
    assert_eq!(quote_review_argument("a'b"), "'a'\"'\"'b'");
}

fn recovery_quantity_report() -> FleetEnsureReport {
    use canic_host::fleet_ensure::model::{
        FleetRecoveryReview, PoolRecoveryFunding, RecoveryDiscovery, RootStartupFundingForecast,
        StartupFundingReuseAssumption,
    };
    let mut report = cycle_quantity_report("rrkah-fqaaa-aaaaa-aaaaq-cai");
    report.plan.recovery_review = Some(Box::new(FleetRecoveryReview {
        base_execution_burn_cycles: 40,
        continuation_reserve_cycles: 60,
        whole_continuation_ceiling_cycles: 120,
        maximum_successor_actions: 8,
        fixture_publication_retry_attempts: 2,
        per_step_burn_cycles: 12,
        startup_funding: vec![RootStartupFundingForecast {
            root: "root".into(),
            startup_minimum_cycles: 40,
            maximum_continuation_steps: 4,
            continuation_allowance_cycles: 48,
            configured_minimum_cycles: 80,
            required_native_cycles: 88,
            reuse_assumption: StartupFundingReuseAssumption::FreshChildrenAndFullPublication,
            unfunded_role: None,
        }],
        known_pool_funding: vec![PoolRecoveryFunding {
            root: "root".into(),
            principal: "asset".into(),
            amount_cycles: u128::from(u64::MAX) + 1,
            ledger_fee_cycles: 7,
            funding_deficit_cycles: u128::from(u64::MAX),
            funding_margin_cycles: 1,
            expected_post_cycles: u128::from(u64::MAX) + 1,
        }],
        discovery: RecoveryDiscovery::PendingCurrentProtocol,
    }));
    report
}

#[test]
fn continuation_forecast_preserves_funding_precision_without_granting_authority() {
    let report = recovery_quantity_report();
    let value = report_json_value(&report).unwrap();
    let continuation = &value["continuation_forecast"];
    assert_eq!(continuation["authority"], "separate_review");
    assert!(continuation["maximum_successor_actions"].is_null());
    assert_eq!(
        continuation["dependent_funding"][0]["amount_cycles"],
        "18446744073709551616"
    );
    assert_eq!(
        continuation["dependent_funding"][0]["ledger_fee_cycles"],
        "7"
    );
    assert!(
        !continuation["requires_live_discovery"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(value["plan"], serde_json::to_value(&report.plan).unwrap());
    assert!(render_text_report(&report).contains("separate review required"));
    for scope in [
        FleetEnsurePlanScope::ReinstallPreparation,
        FleetEnsurePlanScope::RootReinstallPrerequisite,
        FleetEnsurePlanScope::RootStartPrerequisite,
    ] {
        let mut prerequisite = cycle_quantity_report("rrkah-fqaaa-aaaaa-aaaaq-cai");
        prerequisite.plan.scope = scope;
        prerequisite.terminal = true;
        let value = report_json_value(&prerequisite).unwrap();
        let forecast = &value["continuation_forecast"];
        assert_eq!(forecast["authority"], "separate_review");
        assert!(
            !forecast["requires_live_discovery"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn observation_timing_is_informational_and_preserves_failed_call_counts() {
    let timing = canic_host::fleet_ensure::dto::FleetObservationTiming {
        span_id: 1,
        parent_span_id: None,
        identity_lookup_millis: 0,
        stage: canic_host::fleet_ensure::dto::FleetObservationStage::ConfiguredCanisters,
        parent_stage: Some(canic_host::fleet_ensure::dto::FleetObservationStage::FleetSnapshot),
        elapsed_millis: 111,
        remote_call_attempts: 4,
        identity_lookup_attempts: 2,
        cached_read_hits: 7,
        succeeded: Some(false),
    };
    let json: serde_json::Value =
        serde_json::from_str(&render_observation_timing(&timing, true)).unwrap();
    assert_eq!(json["event"], "fleet_ensure_observation");
    assert_eq!(json["observation"]["remote_call_attempts"], 4);
    assert_eq!(json["observation"]["identity_lookup_attempts"], 2);
    assert_eq!(json["observation"]["cached_read_hits"], 7);
    assert_eq!(json["observation"]["parent_stage"], "fleet_snapshot");
    assert!(render_observation_timing(&timing, false).contains("within FleetSnapshot"));
    assert_eq!(json["observation"]["succeeded"], false);
    assert!(json.get("progress").is_none());
}

#[test]
fn native_funding_review_reports_destination_amount_and_approval() {
    let principal = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    let mut report = cycle_quantity_report(principal);
    let digest = "42".repeat(32);
    report.funding_review = Some(FundingReviewRecord {
        operator_mint: None,
        action: EnsureAction::Fund {
            pool_funding: None,
            amount: 95,
            created_at_time: 99,
            expected_post_cycles: 105,
            funding_deficit_cycles: 90,
            funding_margin_cycles: 5,
            ledger: "ledger".into(),
            name: "root-0".into(),
            principal: principal.into(),
        },
        effect: None,
        pause: FundingPauseRecord::Native(NativeFundingRequiredRecord {
            observation_quote: None,
            available_cycles: 10,
            cycles_ledger: "ledger".into(),
            funding_margin_cycles: 5,
            ledger_fee_cycles: 5,
            minimum_cycles: 100,
            operation_id: report.plan.operation_id.clone(),
            plan_sha256: report.plan.plan_sha256.clone(),
            provisioning_action_sha256: "24".repeat(32),
            root: "root-0".into(),
            root_principal: principal.into(),
            shortfall_cycles: 95,
        }),
        review_sha256: digest.clone(),
    });
    let text = render_text_report(&report);
    assert!(text.contains("funding_destination: root_native_balance"));
    assert!(text.contains(&format!("funding_root: {principal}")));
    assert!(text.contains(&format!("funding_apply: --apply {digest}")));
    let json = report_json_value(&report).unwrap();
    assert_eq!(json["funding_review"]["pause"]["kind"], "native");
    assert_eq!(
        json["funding_review"]["pause"]["evidence"]["shortfall_cycles"],
        "95"
    );
    assert_eq!(
        json["funding_review"]["pause"]["evidence"]["ledger_fee_cycles"],
        "5"
    );
    assert!(json["funding_review"]["effect"].is_null());
}

#[test]
fn readiness_requires_explicit_operator_and_has_no_apply_surface() {
    let command = readiness::command();
    assert!(parse_matches(command.clone(), ["staging"].map(OsString::from)).is_err());
    let args = ["staging", "--operator", "rrkah-fqaaa-aaaaa-aaaaq-cai"];
    assert!(parse_matches(command.clone(), args.map(OsString::from)).is_ok());
    let mut apply = args.map(OsString::from).to_vec();
    apply.extend([OsString::from("--apply"), OsString::from("11".repeat(32))]);
    assert!(parse_matches(command, apply).is_err());
}

#[test]
fn concise_apply_success_requires_full_terminal_verification() {
    let mut report = cycle_quantity_report("rrkah-fqaaa-aaaaa-aaaaq-cai");
    report.terminal = true;
    report.plan.scope = FleetEnsurePlanScope::Full;
    assert!(render_apply_report(&report).contains("deployment verified"));
    for scope in [
        FleetEnsurePlanScope::ReinstallPreparation,
        FleetEnsurePlanScope::RootReinstallPrerequisite,
        FleetEnsurePlanScope::RootStartPrerequisite,
    ] {
        report.plan.scope = scope;
        assert_eq!(render_apply_report(&report), render_text_report(&report));
        assert!(!render_apply_report(&report).contains("deployment verified"));
    }
    report.plan.scope = FleetEnsurePlanScope::Full;
    report.terminal = false;
    assert_eq!(render_apply_report(&report), render_text_report(&report));
}

#[test]
fn json_workflow_errors_keep_machine_output_and_typed_source() {
    let error = json_error(FleetCommandError::Io(io::ErrorKind::Interrupted.into()));
    let FleetCommandError::JsonReported { source, .. } = &error else {
        panic!("JSON error boundary")
    };
    assert!(
        matches!(source.as_ref(), FleetCommandError::Io(error) if error.kind() == io::ErrorKind::Interrupted)
    );
    let cli_error = crate::CliError::from(error);
    let rendered = crate::render_cli_error(&cli_error);
    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(json["event"], "fleet_ensure_error");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(crate::cli_error_exit_code(&cli_error), 1);
}

#[test]
fn funding_observation_options_require_an_explicit_root_and_separate_approval() {
    let parse = |args: &[&str]| EnsureOptions::parse(args.iter().map(OsString::from));
    let review = parse(&["ensure", "staging", "--observe-funding", "root-a"]).unwrap();
    assert_eq!(review.observe_funding.as_deref(), Some("root-a"));
    assert!(review.apply.is_none());
    let digest = "31".repeat(32);
    let approved = parse(&[
        "ensure",
        "staging",
        "--observe-funding",
        "root-a",
        "--apply",
        &digest,
    ])
    .unwrap();
    assert_eq!(approved.apply.as_deref(), Some(digest.as_str()));
    for flag in ["--operator-mint", "--reinstall"] {
        assert!(matches!(
            parse(&["ensure", "staging", "--observe-funding", "root-a", flag]),
            Err(FleetCommandError::Usage(_))
        ));
    }
    assert!(matches!(
        parse(&["ensure", "staging", "--observe-funding"]),
        Err(FleetCommandError::Usage(_))
    ));
}

#[test]
fn timing_outcome_retains_exact_plan_scope_and_never_promotes_prerequisite_success() {
    let root = std::env::temp_dir().join(format!("canic-receipt-outcome-{}", std::process::id()));
    let mut report = cycle_quantity_report("aaaaa-aa");
    report.terminal = false;
    let session = progress::ProgressSession::new(true);
    session.retain_receipt(
        &root,
        &progress::receipt::Invocation {
            command: progress::receipt::CommandKind::Ensure,
            fleet: "fleet",
            environment: "local",
            desired_sha256: Some("desired"),
            applied_plan_sha256: Some(&report.plan.plan_sha256),
            reinstall: false,
            next_review_command: "review",
        },
    );
    session.finish(Some(&report));
    drop(session);
    let path = fs::read_dir(root.join(".canic/diagnostics/fleet"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let text = fs::read_to_string(path).unwrap();
    let outcome: serde_json::Value = serde_json::from_str(text.lines().last().unwrap()).unwrap();
    assert_eq!(outcome["data"]["state"], "completed");
    assert_eq!(outcome["data"]["terminal"], false);
    assert_eq!(outcome["data"]["operation_id"], report.plan.operation_id);
    assert_eq!(outcome["data"]["plan_sha256"], report.plan.plan_sha256);
    assert_eq!(
        outcome["data"]["plan_scope"],
        serde_json::to_value(report.plan.scope).unwrap()
    );
    fs::remove_dir_all(root).unwrap();
}
