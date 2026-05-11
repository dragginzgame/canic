use super::*;
use crate::cycles::{
    model::{CycleTopupEventSample, CycleTopupStatus, CycleTrackerPage, CycleTrackerSample},
    options::parse_duration,
    parse::{parse_cycle_tracker_page, parse_cycle_tracker_page_text, parse_topup_event_page},
    render::format_topups,
    transport::{summarize_cycle_tracker, topup_summary_from_events},
};
use canic_host::format::compact_duration;
use canic_host::registry::RegistryEntry;
use canic_host::response_parse::parse_cycle_balance_response;

// Ensure common duration selectors parse into seconds.
#[test]
fn parses_duration_selectors() {
    assert_eq!(parse_duration("30m").expect("30m"), 1_800);
    assert_eq!(parse_duration("6h").expect("6h"), 21_600);
    assert_eq!(parse_duration("7d").expect("7d"), 604_800);
    assert!(matches!(
        parse_duration("0h"),
        Err(CyclesCommandError::InvalidDuration(_))
    ));
}

// Ensure cycle history windows render as compact human durations.
#[test]
fn formats_cycle_history_durations() {
    assert_eq!(compact_duration(0), "0s");
    assert_eq!(compact_duration(45), "45s");
    assert_eq!(compact_duration(90), "1m 30s");
    assert_eq!(compact_duration(7_230), "2h");
    assert_eq!(compact_duration(9_000), "2h 30m");
    assert_eq!(compact_duration(97_200), "1d 3h");
    assert_eq!(compact_duration(1_555_200), "2w 4d");
}

// Ensure cycle tracker JSON output can be parsed from wrapped result shapes.
#[test]
fn parses_cycle_tracker_json() {
    let page = parse_cycle_tracker_page(
        r#"{"Ok":{"entries":[{"timestamp_secs":10,"cycles":"1000"},{"timestamp_secs":"20","cycles":750}],"total":2}}"#,
    )
    .expect("parse page");

    assert_eq!(page.total, 2);
    assert_eq!(page.entries[0].timestamp_secs, 10);
    assert_eq!(page.entries[1].cycles, 750);
}

// Ensure Candid text output remains usable when JSON formatting is unavailable.
#[test]
fn parses_cycle_tracker_candid_text() {
    let page = parse_cycle_tracker_page_text(
        "(variant { 17_724 = record { entries = vec { record { cycles = 1_000 : nat; timestamp_secs = 10 : nat64 }; record { cycles = 750 : nat; timestamp_secs = 20 : nat64 } }; total = 2 : nat64 } })",
    )
    .expect("parse candid page");

    assert_eq!(page.total, 2);
    assert_eq!(page.entries.len(), 2);
    assert_eq!(page.entries[0].cycles, 1_000);
}

// Ensure live cycle balance responses can drive the CURRENT cycles column.
#[test]
fn parses_cycle_balance_response() {
    assert_eq!(
        parse_cycle_balance_response("(variant { 17_724 = 8_200_000_000_000 : nat })"),
        Some(8_200_000_000_000)
    );
    assert_eq!(
        parse_cycle_balance_response("(variant { 17_725 = record { code = 1 : nat } })"),
        None
    );
}

// Ensure top-up event JSON output can be parsed from wrapped result shapes.
#[test]
fn parses_topup_event_json() {
    let page = parse_topup_event_page(
        r#"{"Ok":{"entries":[{"timestamp_secs":10,"sequence":0,"requested_cycles":"4000000000000","transferred_cycles":"4000000000000","status":{"RequestOk":null},"error":null},{"timestamp_secs":"20","sequence":1,"requested_cycles":"4000000000000","transferred_cycles":null,"status":{"RequestErr":null},"error":"no cycles"}],"total":2}}"#,
    )
    .expect("parse topup page");

    assert_eq!(page.total, 2);
    assert_eq!(page.entries[0].status, CycleTopupStatus::RequestOk);
    assert_eq!(page.entries[0].transferred_cycles, Some(4_000_000_000_000));
    assert_eq!(page.entries[1].status, CycleTopupStatus::RequestErr);
}

// Ensure summaries report partial windows when no sample exists before the cutoff.
#[test]
fn summarizes_partial_cycle_window() {
    let entry = RegistryEntry {
        pid: "aaaaa-aa".to_string(),
        role: Some("root".to_string()),
        kind: Some("root".to_string()),
        parent_pid: None,
        module_hash: None,
    };
    let report = summarize_cycle_tracker(
        &entry,
        CycleTrackerPage {
            total: 2,
            entries: vec![
                CycleTrackerSample {
                    timestamp_secs: 100,
                    cycles: 1_000,
                },
                CycleTrackerSample {
                    timestamp_secs: 200,
                    cycles: 700,
                },
            ],
        },
        String::new(),
        50,
        250,
        Some(900),
        None,
    );

    assert_eq!(report.coverage_status, "partial");
    assert_eq!(report.latest_timestamp_secs, Some(250));
    assert_eq!(report.latest_cycles, Some(900));
    assert_eq!(report.delta_cycles, Some(-300));
    assert_eq!(report.rate_cycles_per_hour, Some(-10_800));
}

// Ensure structured top-up events become compact top-up context for cycles output.
#[test]
fn summarizes_topup_events() {
    let entries = vec![
        CycleTopupEventSample {
            timestamp_secs: 100,
            transferred_cycles: Some(4_000_000_000_000),
            status: CycleTopupStatus::RequestOk,
        },
        CycleTopupEventSample {
            timestamp_secs: 200,
            transferred_cycles: Some(4_000_000_000_000),
            status: CycleTopupStatus::RequestOk,
        },
        CycleTopupEventSample {
            timestamp_secs: 10,
            transferred_cycles: Some(4_000_000_000_000),
            status: CycleTopupStatus::RequestOk,
        },
    ];
    let summary = topup_summary_from_events(&entries, 50);

    assert_eq!(summary.request_ok, 2);
    assert_eq!(summary.transferred_cycles, 8_000_000_000_000);
    assert_eq!(format_topups(&summary), "8.00 TC (2)");
}
