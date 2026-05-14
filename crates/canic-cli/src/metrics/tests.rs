use super::*;
use crate::metrics::{
    model::{MetricValue, MetricsKind},
    parse::parse_metrics_page,
    transport::metrics_error_report,
};
use canic_host::registry::RegistryEntry;

// Ensure the public kind selector maps to Candid variant names.
#[test]
fn parses_metric_kind_selectors() {
    assert_eq!(MetricsKind::parse("core").expect("core"), MetricsKind::Core);
    assert_eq!(
        MetricsKind::parse("security")
            .expect("security")
            .candid_variant(),
        "Security"
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

// Ensure zero filtering treats every payload shape consistently.
#[test]
fn detects_zero_metric_values() {
    assert!(MetricValue::Count { count: 0 }.is_zero());
    assert!(
        MetricValue::CountAndU64 {
            count: 0,
            value_u64: 0
        }
        .is_zero()
    );
    assert!(!MetricValue::U128 { value: 1 }.is_zero());
}

// Ensure method-missing responses do not stretch the table with raw ICP output.
#[test]
fn shortens_metrics_unavailable_errors() {
    let entry = RegistryEntry {
        pid: "aaaaa-aa".to_string(),
        role: Some("wasm_store".to_string()),
        kind: Some("wasm_store".to_string()),
        parent_pid: None,
        module_hash: None,
    };
    let report = metrics_error_report(
        &entry,
        "icp command failed\nCanister has no query method 'canic_metrics'.",
    );

    assert_eq!(report.status, "unavailable");
    assert_eq!(report.error.as_deref(), Some("canic_metrics unavailable"));
}
