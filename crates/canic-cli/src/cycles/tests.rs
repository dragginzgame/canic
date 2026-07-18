use super::*;
use crate::cycles::{
    model::{
        CycleTopupEventSample, CycleTopupStatus, CycleTrackerPage, CycleTrackerSample,
        CyclesCanisterStatus, CyclesCoverageStatus, CyclesReport,
    },
    parse::{parse_cycle_tracker_page, parse_topup_event_page},
    transport::summarize_cycle_tracker,
};
use candid::{CandidType, Encode};
use canic_core::{
    cdk::{types::Cycles, utils::hash::hex_bytes},
    dto::{
        cycles::{CycleTopupEvent, CycleTopupEventStatus, CycleTrackerEntry},
        error::Error as CanicError,
        page::Page,
    },
};
use canic_host::format::compact_duration;
use canic_host::registry::RegistryEntry;
use std::ffi::OsString;

// Ensure common duration selectors parse into seconds.
#[test]
fn parses_duration_selectors() {
    for (value, expected) in [("30m", 1_800), ("6h", 21_600), ("7d", 604_800)] {
        let options = options::CyclesOptions::parse_info([
            OsString::from("test"),
            OsString::from("--since"),
            OsString::from(value),
        ])
        .expect("parse cycles duration");

        assert_eq!(options.since_seconds, expected);
    }

    std::assert_matches!(
        options::CyclesOptions::parse_info([
            OsString::from("test"),
            OsString::from("--since"),
            OsString::from("0h"),
        ]),
        Err(CyclesCommandError::Usage(_))
    );
    std::assert_matches!(
        options::CyclesOptions::parse_info([
            OsString::from("test"),
            OsString::from("--limit"),
            OsString::from("0"),
        ]),
        Err(CyclesCommandError::Usage(_))
    );
}

#[test]
fn missing_cycles_deployment_preserves_canonical_typed_error() {
    let error = CyclesCommandError::from(InstalledDeploymentError::NoInstalledDeployment {
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
        CyclesCommandError::InstalledDeployment(
            InstalledDeploymentError::NoInstalledDeployment { .. }
        )
    );
}

// Ensure cycle summaries can target one installed deployment subtree by role or principal.
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

    assert_eq!(options.deployment, "test");
    assert_eq!(options.subtree.as_deref(), Some("scale_hub"));
    assert_eq!(options.since_seconds, 21_600);
    assert_eq!(options.limit, 12);
    assert!(!options.verbose);
}

#[test]
fn cycles_usage_uses_deployment_target_wording() {
    let text = options::info_usage();

    assert!(text.contains("Usage: canic info cycles [OPTIONS] <deployment>"));
    assert!(text.contains("Summarize installed deployment cycle history"));
    assert!(text.contains("Installed deployment target name to inspect"));
    assert!(!text.contains("<fleet>"));
    assert!(!text.contains("Installed fleet"));
}

#[test]
fn cycles_report_json_uses_deployment_identity_field() {
    let value = serde_json::to_value(CyclesReport {
        deployment: "demo-local".to_string(),
        environment: "local".to_string(),
        since_seconds: 86_400,
        generated_at_secs: 1_777_000_000,
        canisters: Vec::new(),
    })
    .expect("serialize cycles report");

    assert_eq!(value["deployment"], "demo-local");
    assert!(value.get("fleet").is_none());
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

#[test]
fn parses_typed_cycle_tracker_page() {
    let output = response_json(&Ok::<_, CanicError>(Page {
        entries: vec![
            CycleTrackerEntry {
                timestamp_secs: 10,
                cycles: Cycles::new(1_000),
            },
            CycleTrackerEntry {
                timestamp_secs: 20,
                cycles: Cycles::new(750),
            },
        ],
        total: 2,
    }));
    let page = parse_cycle_tracker_page(&output).expect("parse page");

    assert_eq!(page.total, 2);
    assert_eq!(page.entries[0].timestamp_secs, 10);
    assert_eq!(page.entries[1].cycles, 750);
}

#[test]
fn parses_typed_topup_event_page() {
    let output = response_json(&Ok::<_, CanicError>(Page {
        entries: vec![
            CycleTopupEvent {
                timestamp_secs: 10,
                sequence: 0,
                requested_cycles: Cycles::new(4_000_000_000_000),
                transferred_cycles: Some(Cycles::new(4_000_000_000_000)),
                status: CycleTopupEventStatus::RequestOk,
                error: None,
            },
            CycleTopupEvent {
                timestamp_secs: 20,
                sequence: 1,
                requested_cycles: Cycles::new(4_000_000_000_000),
                transferred_cycles: None,
                status: CycleTopupEventStatus::RequestErr,
                error: Some("no cycles".to_string()),
            },
        ],
        total: 2,
    }));
    let page = parse_topup_event_page(&output).expect("parse topup page");

    assert_eq!(page.total, 2);
    assert_eq!(page.entries[0].status, CycleTopupStatus::RequestOk);
    assert_eq!(page.entries[0].transferred_cycles, Some(4_000_000_000_000));
    assert_eq!(page.entries[1].status, CycleTopupStatus::RequestErr);
}

fn response_json<T: CandidType>(response: &T) -> String {
    let bytes = Encode!(response).expect("encode response");
    serde_json::json!({ "response_bytes": hex_bytes(bytes) }).to_string()
}

// Ensure summaries report partial windows when no sample exists before the cutoff.
#[test]
fn summarizes_partial_cycle_window() {
    let entry = RegistryEntry {
        pid: "aaaaa-aa".to_string(),
        role: Some("root".to_string()),
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

    assert_eq!(report.status, CyclesCanisterStatus::Ok);
    assert_eq!(report.coverage_status, CyclesCoverageStatus::Partial);
    let value = serde_json::to_value(&report).expect("serialize cycles canister report");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["coverage_status"], "partial");
    assert_eq!(report.latest_timestamp_secs, Some(250));
    assert_eq!(report.latest_cycles, Some(900));
    assert_eq!(report.delta_cycles, Some(-100));
    assert_eq!(report.rate_cycles_per_hour, Some(-2_400));
    assert_eq!(report.burn_cycles, Some(100));
    assert_eq!(report.burn_cycles_per_hour, Some(2_400));
    assert_eq!(report.topup_cycles_per_hour, Some(0));
}

// Ensure structured top-up events become compact top-up context for cycles output.
#[test]
fn summarizes_topup_events() {
    let entry = RegistryEntry {
        pid: "aaaaa-aa".to_string(),
        role: Some("app".to_string()),
        parent_pid: None,
        module_hash: None,
    };
    let report = summarize_cycle_tracker(
        &entry,
        CycleTrackerPage {
            total: 2,
            entries: vec![
                CycleTrackerSample {
                    timestamp_secs: 50,
                    cycles: 10_000_000_000_000,
                },
                CycleTrackerSample {
                    timestamp_secs: 250,
                    cycles: 12_000_000_000_000,
                },
            ],
        },
        String::new(),
        50,
        250,
        None,
        Some(vec![
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
        ]),
    );
    let summary = report.topups.expect("topup summary");

    assert_eq!(summary.request_ok, 2);
    assert_eq!(summary.transferred_cycles, 8_000_000_000_000);
}

// Ensure burn and top-up rates are explicit instead of hidden inside net rate.
#[test]
fn summarizes_burn_and_topup_rates() {
    let entry = RegistryEntry {
        pid: "aaaaa-aa".to_string(),
        role: Some("app".to_string()),
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

// Ensure fresh top-ups are visible before the next hourly tracker sample.
#[test]
fn summarizes_post_sample_topup_events_against_live_balance() {
    let entry = RegistryEntry {
        pid: "aaaaa-aa".to_string(),
        role: Some("app".to_string()),
        parent_pid: None,
        module_hash: None,
    };
    let report = summarize_cycle_tracker(
        &entry,
        CycleTrackerPage {
            total: 1,
            entries: vec![CycleTrackerSample {
                timestamp_secs: 100,
                cycles: 10_000_000_000_000,
            }],
        },
        String::new(),
        0,
        200,
        Some(14_000_000_000_000),
        Some(vec![CycleTopupEventSample {
            timestamp_secs: 150,
            transferred_cycles: Some(5_000_000_000_000),
            status: CycleTopupStatus::RequestOk,
        }]),
    );

    assert_eq!(report.coverage_seconds, Some(100));
    assert_eq!(report.latest_timestamp_secs, Some(200));
    assert_eq!(report.delta_cycles, Some(4_000_000_000_000));
    assert_eq!(
        report.topups.expect("topup summary").transferred_cycles,
        5_000_000_000_000
    );
    assert_eq!(report.topup_cycles_per_hour, Some(180_000_000_000_000));
    assert_eq!(report.burn_cycles, Some(1_000_000_000_000));
    assert_eq!(report.burn_cycles_per_hour, Some(36_000_000_000_000));
}

// Ensure burn inference stays absent when net gain exceeds recorded top-ups.
#[test]
fn omits_burn_when_positive_delta_exceeds_topups() {
    let entry = RegistryEntry {
        pid: "aaaaa-aa".to_string(),
        role: Some("app".to_string()),
        parent_pid: None,
        module_hash: None,
    };
    let report = summarize_cycle_tracker(
        &entry,
        CycleTrackerPage {
            total: 1,
            entries: vec![CycleTrackerSample {
                timestamp_secs: 100,
                cycles: 10_000_000_000_000,
            }],
        },
        String::new(),
        0,
        200,
        Some(16_000_000_000_000),
        Some(vec![CycleTopupEventSample {
            timestamp_secs: 150,
            transferred_cycles: Some(5_000_000_000_000),
            status: CycleTopupStatus::RequestOk,
        }]),
    );

    assert_eq!(report.delta_cycles, Some(6_000_000_000_000));
    assert_eq!(report.topup_cycles_per_hour, Some(180_000_000_000_000));
    assert_eq!(report.burn_cycles, None);
    assert_eq!(report.burn_cycles_per_hour, None);
}
