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
use std::ffi::OsString;

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

// Ensure cycle summaries can target one deployed subtree by role or principal.
#[test]
fn parses_cycles_subtree_option() {
    let options = options::CyclesOptions::parse_info([
        OsString::from("test"),
        OsString::from("--subtree"),
        OsString::from("scale_hub"),
        OsString::from("--since"),
        OsString::from("6h"),
        OsString::from("--limit"),
        OsString::from("12"),
    ])
    .expect("parse cycles subtree options");

    assert_eq!(options.fleet, "test");
    assert_eq!(options.subtree.as_deref(), Some("scale_hub"));
    assert_eq!(options.since_seconds, 21_600);
    assert_eq!(options.limit, 12);
    assert!(!options.verbose);
}

// Ensure verbose cycles output is an explicit opt-in for wider diagnostics.
#[test]
fn parses_cycles_verbose_option() {
    let options =
        options::CyclesOptions::parse_info([OsString::from("test"), OsString::from("--verbose")])
            .expect("parse cycles verbose option");

    assert!(options.verbose);
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

    let page = parse_cycle_tracker_page(
        r#"{"response_candid":"(variant { Ok = record { entries = vec { record { cycles = 1_000 : nat; timestamp_secs = 10 : nat64 } }; total = 1 : nat64 } })"}"#,
    )
    .expect("parse response_candid page");

    assert_eq!(page.total, 1);
    assert_eq!(page.entries[0].cycles, 1_000);
}

#[test]
fn cycle_tracker_json_rejects_malformed_entries_before_response_candid_fallback() {
    assert_eq!(
        parse_cycle_tracker_page(r#"{"Ok":{"entries":[{"timestamp_secs":10}],"total":1}}"#),
        None
    );

    let page = parse_cycle_tracker_page(
        r#"{"Ok":{"entries":[{"timestamp_secs":10}],"total":1},"response_candid":"(variant { Ok = record { entries = vec { record { cycles = 1_000 : nat; timestamp_secs = 10 : nat64 } }; total = 1 : nat64 } })"}"#,
    )
    .expect("fallback to response_candid page");

    assert_eq!(page.entries[0].cycles, 1_000);
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
        parse_cycle_balance_response(
            r#"{"response_candid":"(variant { Ok = 8_200_000_000_000 : nat })"}"#
        ),
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

    let page = parse_topup_event_page(
        r#"{"response_candid":"(variant { Ok = record { entries = vec { record { timestamp_secs = 10 : nat64; transferred_cycles = opt (4_000_000_000_000 : nat); status = variant { RequestOk } } }; total = 1 : nat64 } })"}"#,
    )
    .expect("parse response_candid topup page");

    assert_eq!(page.total, 1);
    assert_eq!(page.entries[0].status, CycleTopupStatus::RequestOk);
    assert_eq!(page.entries[0].transferred_cycles, Some(4_000_000_000_000));
}

#[test]
fn topup_event_json_rejects_malformed_entries_before_response_candid_fallback() {
    assert_eq!(
        parse_topup_event_page(r#"{"Ok":{"entries":[{"timestamp_secs":10}],"total":1}}"#),
        None
    );

    let page = parse_topup_event_page(
        r#"{"Ok":{"entries":[{"timestamp_secs":10}],"total":1},"response_candid":"(variant { Ok = record { entries = vec { record { timestamp_secs = 10 : nat64; transferred_cycles = opt (4_000_000_000_000 : nat); status = variant { RequestOk } } }; total = 1 : nat64 } })"}"#,
    )
    .expect("fallback to response_candid topup page");

    assert_eq!(page.entries[0].status, CycleTopupStatus::RequestOk);
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
        Some(Vec::new()),
    );

    assert_eq!(report.coverage_status, "partial");
    assert_eq!(report.latest_timestamp_secs, Some(250));
    assert_eq!(report.latest_cycles, Some(900));
    assert_eq!(report.delta_cycles, Some(-300));
    assert_eq!(report.rate_cycles_per_hour, Some(-10_800));
    assert_eq!(report.burn_cycles, Some(300));
    assert_eq!(report.burn_cycles_per_hour, Some(10_800));
    assert_eq!(report.topup_cycles_per_hour, Some(0));
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
        CycleTopupEventSample {
            timestamp_secs: 300,
            transferred_cycles: Some(4_000_000_000_000),
            status: CycleTopupStatus::RequestOk,
        },
    ];
    let summary = topup_summary_from_events(&entries, 50, 250);

    assert_eq!(summary.request_ok, 2);
    assert_eq!(summary.transferred_cycles, 8_000_000_000_000);
    assert_eq!(format_topups(&summary), "8.00 TC (2)");
}

// Ensure burn and top-up rates are explicit instead of hidden inside net rate.
#[test]
fn summarizes_burn_and_topup_rates() {
    let entry = RegistryEntry {
        pid: "aaaaa-aa".to_string(),
        role: Some("app".to_string()),
        kind: Some("singleton".to_string()),
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
                    cycles: 10_000_000_000_000,
                },
                CycleTrackerSample {
                    timestamp_secs: 3_700,
                    cycles: 8_000_000_000_000,
                },
            ],
        },
        String::new(),
        100,
        3_700,
        None,
        Some(vec![CycleTopupEventSample {
            timestamp_secs: 1_000,
            transferred_cycles: Some(5_000_000_000_000),
            status: CycleTopupStatus::RequestOk,
        }]),
    );

    assert_eq!(report.delta_cycles, Some(-2_000_000_000_000));
    assert_eq!(report.rate_cycles_per_hour, Some(-2_000_000_000_000));
    assert_eq!(report.topup_cycles_per_hour, Some(5_000_000_000_000));
    assert_eq!(report.burn_cycles, Some(7_000_000_000_000));
    assert_eq!(report.burn_cycles_per_hour, Some(7_000_000_000_000));
}
