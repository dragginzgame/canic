use super::*;
use crate::metrics::{
    model::{MetricValue, MetricsKind},
    parse::parse_metrics_page,
};

// Ensure the public kind selector accepts the expected CLI vocabulary.
#[test]
fn parses_metric_kind_selectors() {
    assert_eq!(MetricsKind::parse("core").expect("core"), MetricsKind::Core);
    assert_eq!(
        MetricsKind::parse("security").expect("security"),
        MetricsKind::Security
    );
    assert!(matches!(
        MetricsKind::parse("cycles"),
        Err(MetricsCommandError::InvalidKind(_))
    ));
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

// Ensure ICP CLI response wrappers without did metadata still parse.
#[test]
fn parses_metrics_response_candid_text() {
    let entries = parse_metrics_page(
        r#"{"response_candid":"(\n  variant {\n    Ok = record {\n      total = 2 : nat64;\n      entries = vec {\n        record {\n          \"principal\" = null;\n          value = variant { Count = 1 : nat64 };\n          labels = vec { \"canister_ops\"; \"create\"; \"app\"; \"completed\"; \"ok\" };\n        };\n        record {\n          \"principal\" = opt principal \"aaaaa-aa\";\n          value = variant { CountAndU64 = record { count = 3 : nat64; value_u64 = 12 : nat64 } };\n          labels = vec { \"timer\"; \"tick\" };\n        };\n      };\n    }\n  },\n)"}"#,
    )
    .expect("parse response_candid metrics page");

    assert_eq!(entries.len(), 2);
    assert_eq!(
        entries[0].labels,
        ["canister_ops", "create", "app", "completed", "ok"]
    );
    assert_eq!(entries[0].value, MetricValue::Count { count: 1 });
    assert_eq!(entries[1].principal.as_deref(), Some("aaaaa-aa"));
    assert_eq!(
        entries[1].value,
        MetricValue::CountAndU64 {
            count: 3,
            value_u64: 12
        }
    );
}

#[test]
fn metrics_json_rejects_malformed_entries_before_response_candid_fallback() {
    assert_eq!(
        parse_metrics_page(
            r#"{"Ok":{"entries":[{"labels":["timer"],"principal":null}],"total":1}}"#
        ),
        None
    );

    let entries = parse_metrics_page(
        r#"{"Ok":{"entries":[{"labels":["timer"],"principal":null}],"total":1},"response_candid":"(\n  variant {\n    Ok = record {\n      total = 1 : nat64;\n      entries = vec {\n        record {\n          \"principal\" = null;\n          value = variant { Count = 1 : nat64 };\n          labels = vec { \"timer\"; \"tick\" };\n        };\n      };\n    }\n  },\n)"}"#,
    )
    .expect("fallback to response_candid metrics page");

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].labels, ["timer", "tick"]);
}
