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
            "--json"
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
    assert!(render(&report).contains("No publication evidence found."));
    report.publications.push(FleetArtifact {
        operation: "01".repeat(32),
        publication_operation: "02".repeat(32),
        deployment: DeploymentName {
            app: "demo".into(),
            fleet: "local".into(),
            canister: "app".into(),
        },
        publication: Publication {
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
    assert!(render(&report).contains("unconfirmed"));
    report.publications[0].publication_retained = true;
    assert!(render(&report).contains("published"));
    report.issues.push("bad\u{1b}[2Jrecord".into());
    let text = render(&report);
    assert!(!text.contains('\u{1b}'));
    assert!(matches!(output(&report, false), Output::Partial(_)));
    let Output::Partial(json) = output(&report, true) else {
        panic!("partial JSON")
    };
    let Output::Json(value) = *json else {
        panic!("JSON")
    };
    assert_eq!(value["publications"][0]["operation"], "01".repeat(32));
    assert_eq!(value["issues"].as_array().unwrap().len(), 1);
}
