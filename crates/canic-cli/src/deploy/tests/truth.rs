use super::super::resume_report as deploy_resume_report;
use super::super::truth as deploy_truth;
use super::*;

fn resume_report_receipt_args() -> Vec<OsString> {
    vec![
        OsString::from("--receipt"),
        OsString::from("receipt.json"),
        OsString::from("demo"),
    ]
}

#[test]
fn deploy_leaf_commands_parse_like_check() {
    for (command, usage, name) in truth_leaf_commands() {
        let options = DeployTruthOptions::parse([OsString::from("demo")], command, usage)
            .unwrap_or_else(|_| panic!("parse deploy {name}"));

        assert_eq!(options.fleet, "demo");
    }

    let resume_report =
        deploy_resume_report::DeployResumeReportOptions::parse(resume_report_receipt_args())
            .expect("parse deploy resume-report");

    assert_eq!(resume_report.truth.fleet, "demo");
    assert_eq!(resume_report.receipt, Some(PathBuf::from("receipt.json")));
}

#[test]
fn deploy_resume_report_allows_catalog_resolved_receipt_lookup() {
    let resume_report =
        deploy_resume_report::DeployResumeReportOptions::parse([OsString::from("demo")])
            .expect("parse deploy resume-report");

    assert_eq!(resume_report.truth.fleet, "demo");
    assert_eq!(resume_report.receipt, None);
}

#[test]
fn deploy_resume_report_resolves_latest_receipt_by_canonical_fleet_identity() {
    let root = super::fixtures::temp_json_path("canonical-fleet-receipt");
    fs::create_dir_all(&root).expect("create ICP root");
    let network = canic_core::ids::CanonicalNetworkId::ic_mainnet();
    let fleet_id = canic_core::ids::FleetId::from_generated_bytes([7; 32]);
    let catalog_path = root
        .join(".canic")
        .join("networks")
        .join(network.to_string())
        .join("fleets/catalog.json");
    fs::create_dir_all(catalog_path.parent().expect("catalog parent"))
        .expect("create catalog parent");
    fs::write(
        &catalog_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1,
            "canonical_network_id": network,
            "entries": [{
                "canonical_network_id": network,
                "fleet_id": fleet_id,
                "fleet_name": "demo",
                "app": "demo",
                "environment": "ic",
                "deployed_at_unix_secs": 1,
                "coordinator_principal": "rrkah-fqaaa-aaaaa-aaaaq-cai",
            }],
        }))
        .expect("encode Fleet catalog"),
    )
    .expect("write Fleet catalog");
    let receipt_dir = root
        .join(".canic")
        .join("networks")
        .join(network.to_string())
        .join("fleets")
        .join(fleet_id.to_string())
        .join("deployment-receipts");
    fs::create_dir_all(&receipt_dir).expect("create receipt directory");
    let older = receipt_dir.join("unix_1-check.json");
    let latest = receipt_dir.join("unix_2-check.json");
    fs::write(&older, "{}").expect("write older receipt");
    fs::write(&latest, "{}").expect("write latest receipt");

    let options = deploy_resume_report::DeployResumeReportOptions {
        truth: DeployTruthOptions {
            fleet: "demo".to_string(),
            environment: "ic".to_string(),
            profile: None,
        },
        receipt: None,
    };

    assert_eq!(
        options
            .receipt_path_from_root(&root)
            .expect("resolve latest receipt"),
        latest
    );

    fs::remove_dir_all(root).expect("remove fixture");
}

type TruthCommandFactory = fn() -> ClapCommand;
type TruthUsageFactory = fn() -> String;

fn truth_leaf_commands() -> [(TruthCommandFactory, TruthUsageFactory, &'static str); 4] {
    [
        (deploy_truth::plan_command, deploy_truth::plan_usage, "plan"),
        (
            deploy_truth::inventory_command,
            deploy_truth::inventory_usage,
            "inventory",
        ),
        (deploy_truth::diff_command, deploy_truth::diff_usage, "diff"),
        (
            deploy_truth::report_command,
            deploy_truth::report_usage,
            "report",
        ),
    ]
}
