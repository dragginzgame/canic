use super::*;
use crate::args::Cli;
use canic_host::{Artifact, DeploymentName, FleetArtifact, Network, Publication, UpgradeTarget};
use clap::Parser as _;

#[test]
fn artifact_list_has_its_own_scope_and_no_effect_arguments() {
    assert!(
        Cli::try_parse_from([
            "canic",
            "fleet",
            "artifact",
            "list",
            "--app",
            "demo",
            "--fleet",
            "local",
            "--state-dir",
            "journal",
            "--config",
            "demo.json",
            "--json",
            "--details"
        ])
        .is_ok()
    );
    for extra in ["--identity", "--canister", "--no-config", "--open"] {
        assert!(Cli::try_parse_from(["canic", "fleet", "artifact", "list", extra]).is_err());
    }
    assert!(Cli::try_parse_from(["canic", "fleet", "artifacts"]).is_err());
}

#[test]
fn listing_distinguishes_publication_evidence_and_retains_partial_results() {
    let mut report = FleetArtifacts {
        app: "demo".into(),
        fleet: "local".into(),
        network: Network::Mainnet,
        scope: "local publication evidence",
        publications: vec![],
        issues: vec![],
    };
    assert!(render(&report, false).contains("No publication evidence found."));
    report.publications.push(FleetArtifact {
        operation: "01".repeat(32),
        publication_operation: "02".repeat(32),
        deployment: DeploymentName {
            app: "demo".into(),
            fleet: "local".into(),
            canister: "app".into(),
        },
        publication: Publication {
            after_removal: None,
            root: UpgradeTarget {
                canister: "root".into(),
                module: "11".repeat(32),
            },
            store: UpgradeTarget {
                canister: "store".into(),
                module: "22".repeat(32),
            },
            release: serde_json::from_value(serde_json::json!({"name":"app", "version":"v1"}))
                .unwrap(),
        },
        artifact: Artifact {
            sha256: "33".repeat(32),
            bytes: 8,
        },
        publication_retained: false,
    });
    assert!(render(&report, false).contains("unconfirmed"));
    report.publications[0].publication_retained = true;
    assert!(render(&report, false).contains("published"));
    assert!(!render(&report, false).contains(&"01".repeat(32)));
    let detailed = render(&report, true);
    assert!(detailed.contains(&format!("Operation: {}", "01".repeat(32))));
    assert!(detailed.contains(&format!("Nested publication approval: {}", "02".repeat(32))));
    assert!(detailed.contains(&format!("Wasm SHA-256: {}", "33".repeat(32))));
    report.publications[0].publication.root.canister = "root\u{1b}[2J".into();
    report.issues.push("bad\u{1b}[2Jrecord".into());
    let text = render(&report, true);
    assert!(!text.contains('\u{1b}'));
    assert!(matches!(output(&report, false, true), Output::Partial(_)));
    let Output::Partial(json) = output(&report, true, true) else {
        panic!("partial JSON")
    };
    let Output::Json(value) = *json else {
        panic!("JSON")
    };
    assert_eq!(value["publications"][0]["operation"], "01".repeat(32));
    assert_eq!(value["issues"].as_array().unwrap().len(), 1);
    let entry = &mut report.publications[0];
    entry.publication_operation.clone_from(&entry.operation);
    assert!(!render(&report, true).contains("Nested publication approval"));
}

#[test]
fn removal_and_restoration_plan_commands_require_exact_operations_and_reject_effect_options() {
    for (action, extra) in [
        ("plan-removal", vec!["--action", "delete"]),
        ("plan-removal", vec!["--action", "cancel"]),
        ("plan-restoration", vec![]),
    ] {
        let mut args = vec![
            "canic",
            "fleet",
            "artifact",
            action,
            "OPERATION",
            "--app",
            "demo",
            "--fleet",
            "local",
        ];
        args.extend(extra);
        assert!(Cli::try_parse_from(&args).is_ok());
        for flag in ["--identity", "--icp-bin", "--wasm", "--open"] {
            let mut invalid = args.clone();
            invalid.extend([flag, "unexpected"]);
            assert!(Cli::try_parse_from(invalid).is_err());
        }
    }
    assert!(
        Cli::try_parse_from(["canic", "fleet", "artifact", "plan-removal", "OPERATION"]).is_err()
    );
}
