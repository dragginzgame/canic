// Ensure the public kind selector accepts the expected CLI vocabulary.

use super::*;
use crate::metrics::{
    model::{MetricValue, MetricsKind, MetricsReport},
    parse::metric_page,
};
use candid::Principal;
use canic_contracts::dto::{
    metrics::{MetricEntry as MetricEntryDto, MetricValue as MetricValueDto},
    page::Page,
};

#[test]
fn parses_metric_kind_selectors() {
    let options =
        MetricsOptions::parse_info([OsString::from("test")]).expect("default metrics kind");
    assert_eq!(options.fleet, "test");
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
fn metrics_usage_uses_fleet_wording() {
    let text = info_usage();

    assert!(text.contains("Usage: canic info metrics [OPTIONS] <fleet>"));
    assert!(text.contains("Installed Fleet name to inspect"));
    assert!(text.contains("--verbose"));
}

#[test]
fn metrics_report_json_uses_fleet_identity_field() {
    let value = serde_json::to_value(MetricsReport {
        fleet: "demo-local".to_string(),
        environment: "local".to_string(),
        kind: MetricsKind::Core,
        canisters: Vec::new(),
    })
    .expect("serialize metrics report");

    assert_eq!(value["fleet"], "demo-local");
}

#[test]
fn missing_metrics_fleet_preserves_canonical_typed_error() {
    let error = MetricsCommandError::from(CurrentFleetInventoryError::NotConverged {
        environment: "local".to_string(),
        fleet: "demo-local".to_string(),
    });
    std::assert_matches!(
        error,
        MetricsCommandError::CurrentFleet(CurrentFleetInventoryError::NotConverged {
            environment,
            fleet,
        }) if environment == "local" && fleet == "demo-local"
    );
}

#[test]
fn parses_typed_metrics_page() {
    let page = Page {
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
                value: MetricValueDto::Measurement {
                    count: 3,
                    total: u128::from(u64::MAX) + 1,
                    count_overflowed: false,
                    total_overflowed: false,
                },
            },
        ],
        total: 3,
    };
    let wire = candid::encode_one(page).expect("encode metrics page");
    let page = candid::decode_one(&wire).expect("decode metrics page");
    let entries = metric_page(page);

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].labels, ["lifecycle", "init", "started"]);
    assert_eq!(entries[0].value, MetricValue::Count { count: 2 });
    assert_eq!(entries[1].principal.as_deref(), Some("aaaaa-aa"));
    assert_eq!(entries[1].value, MetricValue::U128 { value: 1_000 });
    assert_eq!(
        entries[2].value,
        MetricValue::Measurement {
            count: 3,
            total: u128::from(u64::MAX) + 1,
            count_overflowed: false,
            total_overflowed: false
        }
    );
}

#[test]
fn metric_measurement_quality_survives_candid_and_json() {
    for (count, total, count_overflowed, total_overflowed) in [
        (0, 0, false, false),
        (1, 0, false, false),
        (u64::MAX, u128::from(u64::MAX), false, false),
        (2, u128::from(u64::MAX) + 1, false, false),
        (u64::MAX, 0, true, false),
        (u64::MAX, u128::MAX, true, true),
    ] {
        let page = Page {
            entries: vec![MetricEntryDto {
                labels: vec!["perf".into(), "instructions".into()],
                principal: None,
                value: MetricValueDto::Measurement {
                    count,
                    total,
                    count_overflowed,
                    total_overflowed,
                },
            }],
            total: 1,
        };
        let wire = candid::encode_one(page).unwrap();
        let rows = metric_page(candid::decode_one(&wire).unwrap());
        let json = serde_json::to_string(&rows).unwrap();
        assert!(json.contains(&format!("\"total\":{total}")));
        assert!(json.contains(&format!("\"count_overflowed\":{count_overflowed}")));
        assert!(json.contains(&format!("\"total_overflowed\":{total_overflowed}")));
        assert_eq!(
            rows[0].value,
            MetricValue::Measurement {
                count,
                total,
                count_overflowed,
                total_overflowed
            }
        );
    }
}
