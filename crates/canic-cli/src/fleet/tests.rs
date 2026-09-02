use super::*;
use crate::test_support::temp_dir;
use canic_host::fleet_ensure::model::{
    ActualCycleConservation, CanisterDisposition, CanisterPlan, CycleConservation, EnsureAction,
    FleetEnsurePlan, FleetEnsurePlanScope,
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
fn ensure_reopens_retained_reviewed_input_when_working_toml_is_missing() {
    use canic_host::fleet_ensure::{
        model::{
            CanisterRuntimeStatus, DesiredFleet, DesiredFleetArtifacts, FleetEnsureCompletion,
            FleetEnsureJournalRecord, FleetObservation, LiveCanister,
        },
        ops::{EnsurePaths, write_journal, write_plan},
        policy::compile_plan,
    };

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
            ledger_fee_cycles: 0,
            operator_cycles: 0,
            protocol_ready: BTreeMap::new(),
        },
        1,
    )
    .expect("compile retained desired authority");
    let paths = EnsurePaths::under(&root, "local", "retained");
    write_plan(&paths, &plan).expect("retain reviewed plan");
    write_journal(
        &paths,
        &FleetEnsureJournalRecord {
            completion: FleetEnsureCompletion::InProgress,
            effects: Vec::new(),
            fleet: "retained".to_string(),
            initial_controlled_cycles: 20,
            initial_operator_cycles: 0,
            operation_id: plan.operation_id.clone(),
            plan_sha256: plan.plan_sha256.clone(),
            schema_version: 1,
            stalled_observations: 0,
        },
    )
    .expect("retain in-progress journal");
    let options = EnsureOptions {
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

#[test]
fn text_report_formats_every_cycle_quantity_with_three_decimal_units() {
    let principal = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    let report = FleetEnsureReport {
        actual_conservation: Some(ActualCycleConservation {
            exact_unavoidable_fee_cycles: 3_500_700_000_000,
            final_controlled_cycles: 1_001_498_000_000_000,
            measured_execution_burn_cycles: 2_000_000_000,
            observed_starting_cycles: 1_000_000_000_000_000,
            operator_debit_cycles: 1_500_000_000_000,
            received_new_funding_cycles: 1_500_000_000_000,
        }),
        effects_applied: 1,
        plan: FleetEnsurePlan {
            canisters: vec![CanisterPlan {
                actions: vec![EnsureAction::Fund {
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
                expected_post_operation_cycles: 93_600_000_000_000,
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
            root_start_authority: None,
            reviewed_desired: None,
            schema_version: 1,
            scope: FleetEnsurePlanScope::Full,
            terminal_inventory_operation_id: None,
        },
        terminal: false,
    };

    assert_eq!(
        render_text_report(&report),
        format!(
            "fleet: demo\
             \noperation_id: operation\
             \nplan_sha256: plan\
             \nplan_scope: full\
             \nterminal: false\
             \nobserved_controlled_cycles: 0.000B\
             \nretained_in_reused_canisters_cycles: 0.000B\
             \nscheduled_transfer_cycles: 0.000B\
             \nmaximum_unavoidable_fee_cycles: 3.501T\
             \nmaximum_execution_burn_cycles: 82.000T\
             \nmaximum_new_funding_cycles: 175.600T\
             \nmaximum_operator_debit_cycles: 179.101T\
             \nexpected_post_operation_cycles: 93.600T\
             \ncanisters:\
             \n  app: disposition=Reuse principal={principal} observed_cycles=1.250B effects=1\
             \n  native_topup app: cycles_ledger_withdraw=1.000Q ledger=ledger target={principal} deficit=2.000B margin=0.500B expected_native_post=1.002T\
             \nconservation_equation: 0.000B + 179.101T - 3.501T - 82.000T = 93.600T\
             \nmeasured_conservation: 1.000Q + 1.500T - 2.000B = 1.001Q"
        )
    );
}
