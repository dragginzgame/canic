use super::*;
use crate::test_support::temp_dir;
use canic_host::fleet_ensure::{
    model::{
        ActualCycleConservation, CanisterDisposition, CanisterPlan, CanisterRuntimeStatus,
        CycleConservation, DesiredFleet, DesiredFleetArtifacts, EnsureAction,
        EstateFundingDomainPlan, FleetEnsureCompletion, FleetEnsureJournalRecord, FleetEnsurePlan,
        FleetEnsurePlanScope, FleetObservation, LiveCanister,
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
    assert_eq!(names, ["ensure", "generate"]);
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
    let plan = compile_plan(
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
    let options = EnsureOptions {
        reinstall: false,
        apply: Some(plan.plan_sha256),
        desired: PathBuf::from("missing.toml"),
        environment: Some("local".to_string()),
        fleet: "retained".to_string(),
        icp: "icp".to_string(),
        json: false,
    };

    let loaded = load_ensure_authority(&root, &root.join("missing.toml"), &options)
        .expect("load exact retained desired without working TOML");
    assert_eq!(loaded.desired, desired);
    assert_eq!(loaded.sha256, desired_sha256);

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
            measured_execution_burn_cycles: 2_000_000_000,
            observed_starting_cycles: 1_000_000_000_000_000,
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
             \nmeasured_conservation: 1.000Q observed starting + 1.500T received funding - 500.000B exact Root-funded creation fees - 2.000B measured execution burn = 1.001Q final controlled"
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
    assert!(!text.contains("measured_conservation:"));
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
fn phase_progress_json_has_exact_operation_authority_and_numeric_counts() {
    let progress = FleetEnsureProgress {
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
        FleetEnsureSuccessorReviewReason, FleetRecoveryReview, FleetReviewAction,
        FleetSuccessorReview, RecoveryDiscovery,
    };
    let mut report = cycle_quantity_report("rrkah-fqaaa-aaaaa-aaaaq-cai");
    report.plan.recovery_review = Some(Box::new(FleetRecoveryReview {
        base_execution_burn_cycles: 40,
        continuation_reserve_cycles: 60,
        whole_continuation_ceiling_cycles: 120,
        known_pool_funding: Vec::new(),
        discovery: RecoveryDiscovery::PendingCurrentProtocol,
    }));
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
    let progress = FleetEnsureProgress {
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

#[test]
fn observation_timing_is_informational_and_preserves_failed_call_counts() {
    let timing = canic_host::fleet_ensure::dto::FleetObservationTiming {
        stage: canic_host::fleet_ensure::dto::FleetObservationStage::ConfiguredCanisters,
        elapsed_millis: 111,
        remote_call_attempts: 4,
        succeeded: false,
    };
    let json: serde_json::Value =
        serde_json::from_str(&render_observation_timing(&timing, true)).unwrap();
    assert_eq!(json["event"], "fleet_ensure_observation");
    assert_eq!(json["observation"]["remote_call_attempts"], 4);
    assert_eq!(json["observation"]["succeeded"], false);
    assert!(json.get("progress").is_none());
}
