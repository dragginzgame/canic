use super::*;
use crate::metrics::{
    model::{MetricValue, MetricsKind, MetricsReport},
    parse::parse_metrics_page,
};

// Ensure the public kind selector accepts the expected CLI vocabulary.
#[test]
fn parses_metric_kind_selectors() {
    let options =
        MetricsOptions::parse_info([OsString::from("test")]).expect("default metrics kind");
    assert_eq!(options.deployment, "test");
    assert_eq!(options.kind, MetricsKind::Core);
    assert!(!options.verbose);

    let options = MetricsOptions::parse_info([
        OsString::from("test"),
        OsString::from("--kind"),
        OsString::from("security"),
    ])
    .expect("security metrics kind");
    assert_eq!(options.kind, MetricsKind::Security);

    std::assert_matches!(
        MetricsOptions::parse_info([
            OsString::from("test"),
            OsString::from("--kind"),
            OsString::from("cycles"),
        ]),
        Err(MetricsCommandError::Usage(_))
    );

    std::assert_matches!(
        MetricsOptions::parse_info([
            OsString::from("test"),
            OsString::from("--limit"),
            OsString::from("banana"),
        ]),
        Err(MetricsCommandError::Usage(_))
    );
    std::assert_matches!(
        MetricsOptions::parse_info([
            OsString::from("test"),
            OsString::from("--limit"),
            OsString::from("0"),
        ]),
        Err(MetricsCommandError::Usage(_))
    );
}

// Ensure verbose metrics output is an explicit opt-in for wider diagnostics.
#[test]
fn parses_metrics_verbose_option() {
    let options = MetricsOptions::parse_info([OsString::from("test"), OsString::from("--verbose")])
        .expect("parse metrics verbose option");

    assert!(options.verbose);

    let options = MetricsOptions::parse_info([OsString::from("test"), OsString::from("-v")])
        .expect("parse metrics short verbose option");

    assert!(options.verbose);
}

#[test]
fn metrics_usage_uses_deployment_target_wording() {
    let text = info_usage();

    assert!(text.contains("Usage: canic info metrics [OPTIONS] <deployment>"));
    assert!(text.contains("Installed deployment target name to inspect"));
    assert!(text.contains("--verbose"));
    assert!(!text.contains("<fleet>"));
    assert!(!text.contains("Installed fleet"));
}

#[test]
fn metrics_report_json_uses_deployment_identity_field() {
    let value = serde_json::to_value(MetricsReport {
        deployment: "demo-local".to_string(),
        network: "local".to_string(),
        kind: MetricsKind::Core,
        canisters: Vec::new(),
    })
    .expect("serialize metrics report");

    assert_eq!(value["deployment"], "demo-local");
    assert!(value.get("fleet").is_none());
}

#[test]
fn missing_metrics_deployment_mentions_unverified_registration_acknowledgement() {
    let message = MetricsCommandError::NoInstalledDeployment {
        network: "local".to_string(),
        deployment: "demo-local".to_string(),
    }
    .to_string();

    assert!(message.contains("canic deploy register demo-local"));
    assert!(message.contains("--allow-unverified"));
}

// Ensure named JSON metric pages parse into the CLI row shape.
#[test]
fn parses_metrics_json_page() {
    let entries = parse_metrics_page(
        r#"{"Ok":{"entries":[{"labels":["lifecycle","init","started"],"principal":null,"value":{"Count":2}},{"labels":["cycles_funding","minted"],"principal":"aaaaa-aa","value":{"U128":"1000"}},{"labels":["timer","tick"],"principal":null,"value":{"CountAndU64":{"count":3,"value_u64":12}}}],"total":3}}"#,
    )
    .expect("parse metrics page");

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].labels, ["lifecycle", "init", "started"]);
    assert_eq!(entries[0].value, MetricValue::Count { count: 2 });
    assert_eq!(entries[1].principal.as_deref(), Some("aaaaa-aa"));
    assert_eq!(entries[1].value, MetricValue::U128 { value: 1_000 });
    assert_eq!(
        entries[2].value,
        MetricValue::CountAndU64 {
            count: 3,
            value_u64: 12
        }
    );
}

#[test]
fn metrics_json_rejects_malformed_entries() {
    assert_eq!(
        parse_metrics_page(
            r#"{"Ok":{"entries":[{"labels":["timer"],"principal":null}],"total":1}}"#
        ),
        None
    );
}
