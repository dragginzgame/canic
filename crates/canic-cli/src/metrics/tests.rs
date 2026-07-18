use super::*;
use crate::metrics::{
    model::{MetricValue, MetricsKind, MetricsReport},
    parse::parse_metrics_page,
};
use candid::{CandidType, Encode, Principal};
use canic_core::{
    cdk::utils::hash::hex_bytes,
    dto::{
        error::Error as CanicError,
        metrics::{MetricEntry as MetricEntryDto, MetricValue as MetricValueDto},
        page::Page,
    },
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
        environment: "local".to_string(),
        kind: MetricsKind::Core,
        canisters: Vec::new(),
    })
    .expect("serialize metrics report");

    assert_eq!(value["deployment"], "demo-local");
    assert!(value.get("fleet").is_none());
}

#[test]
fn missing_metrics_deployment_preserves_canonical_typed_error() {
    let error = MetricsCommandError::from(InstalledDeploymentError::NoInstalledDeployment {
        environment: "local".to_string(),
        deployment: "demo-local".to_string(),
    });
    let message = error.to_string();

    assert_eq!(
        message,
        "deployment target demo-local is not installed on environment local"
    );
    std::assert_matches!(
        error,
        MetricsCommandError::InstalledDeployment(
            InstalledDeploymentError::NoInstalledDeployment { .. }
        )
    );
}

#[test]
fn parses_typed_metrics_page() {
    let output = response_json(&Ok::<_, CanicError>(Page {
        entries: vec![
            MetricEntryDto {
                labels: vec!["lifecycle".into(), "init".into(), "started".into()],
                principal: None,
                value: MetricValueDto::Count(2),
            },
            MetricEntryDto {
                labels: vec!["cycles_funding".into(), "minted".into()],
                principal: Some(Principal::from_text("aaaaa-aa").expect("principal")),
                value: MetricValueDto::U128(1_000),
            },
            MetricEntryDto {
                labels: vec!["timer".into(), "tick".into()],
                principal: None,
                value: MetricValueDto::CountAndU64 {
                    count: 3,
                    value_u64: 12,
                },
            },
        ],
        total: 3,
    }));
    let entries = parse_metrics_page(&output).expect("parse metrics page");

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

fn response_json<T: CandidType>(response: &T) -> String {
    let bytes = Encode!(response).expect("encode response");
    serde_json::json!({ "response_bytes": hex_bytes(bytes) }).to_string()
}
