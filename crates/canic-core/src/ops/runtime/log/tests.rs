use super::*;

fn append_entry(
    selected: &mut Option<RuntimeLog>,
    time: u64,
    source: &str,
    topic: Option<&str>,
    level: ic_logging::Level,
) {
    append(
        selected,
        ic_logging::EntryInput {
            created_at_secs: time,
            level,
            source,
            topic,
            message: "🦀 diagnostic text",
        },
        2_000,
        16,
    )
    .expect("valid fixture append");
}

#[test]
fn canic_topic_and_severity_project_through_the_upstream_filter() {
    let mut selected = None;
    append_entry(
        &mut selected,
        1,
        "worker",
        Some(Topic::Cycles.log_label()),
        ic_logging::Level::Info,
    );
    append_entry(
        &mut selected,
        2,
        "other",
        Some(Topic::Cycles.log_label()),
        ic_logging::Level::Error,
    );
    append_entry(
        &mut selected,
        3,
        "worker",
        Some(Topic::Cycles.log_label()),
        ic_logging::Level::Warn,
    );
    let runtime = selected.unwrap();
    let page = project_page(
        &runtime.log,
        ic_logging::Filter {
            source: Some("worker"),
            topic: Some(Topic::Cycles.log_label()),
            min_level: Some(upstream_level(Level::Warn)),
        },
        PageRequest {
            limit: 10,
            offset: 0,
        },
    );
    assert_eq!(page.total, 1);
    assert_eq!(page.entries.len(), 1);
    let row = &page.entries[0];
    assert_eq!(row.crate_name, "worker");
    assert_eq!(row.topic.as_deref(), Some("cycles"));
    assert_eq!(row.level, Level::Warn);
    assert_eq!(row.created_at, 3);
    assert!(row.message.len() <= 16);
}

#[test]
fn offset_and_exact_totals_cross_upstream_scan_batches_including_empty_matches() {
    let mut selected = None;
    for time in 0..800 {
        let source = if time < 300 { "worker" } else { "other" };
        append_entry(&mut selected, time, source, None, ic_logging::Level::Info);
    }
    let runtime = selected.unwrap();
    let filter = ic_logging::Filter {
        source: Some("worker"),
        ..ic_logging::Filter::default()
    };
    let page = project_page(
        &runtime.log,
        filter,
        PageRequest {
            limit: 2,
            offset: 298,
        },
    );
    assert_eq!(page.total, 300);
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.created_at)
            .collect::<Vec<_>>(),
        [1, 0]
    );
    let beyond = project_page(
        &runtime.log,
        filter,
        PageRequest {
            limit: 10,
            offset: u64::MAX,
        },
    );
    assert_eq!(beyond.entries.len(), 0);
    assert_eq!(beyond.total, 300);
    let count_only = project_page(
        &runtime.log,
        filter,
        PageRequest {
            limit: 0,
            offset: 0,
        },
    );
    assert_eq!(count_only.entries.len(), 0);
    assert_eq!(count_only.total, 300);
}

#[test]
fn canic_reply_limit_is_independent_of_the_upstream_scan_budget() {
    let mut selected = None;
    for time in 0..1_100 {
        append_entry(&mut selected, time, "worker", None, ic_logging::Level::Info);
    }
    let runtime = selected.unwrap();
    let page = project_page(
        &runtime.log,
        ic_logging::Filter::default(),
        PageRequest {
            limit: u64::MAX,
            offset: 0,
        },
    );
    assert_eq!(page.total, 1_100);
    assert_eq!(page.entries.len(), LOG_PAGE_MAX_LIMIT);
    assert_eq!(page.entries[0].created_at, 1_099);
    assert_eq!(page.entries[LOG_PAGE_MAX_LIMIT - 1].created_at, 100);
}

#[test]
fn disabled_logging_owns_no_recorder_and_projects_an_empty_page() {
    LogOps::reset_for_tests();
    let mut selected = None;
    let result = append(
        &mut selected,
        ic_logging::EntryInput {
            created_at_secs: 1,
            level: ic_logging::Level::Info,
            source: "worker",
            topic: None,
            message: "disabled",
        },
        0,
        16,
    );
    assert_eq!(result.unwrap(), None);
    assert!(selected.is_none());
    let page = LogOps::page_filtered(
        None,
        None,
        None,
        PageRequest {
            limit: 10,
            offset: 0,
        },
    );
    assert_eq!(page.total, 0);
    assert_eq!(page.entries.len(), 0);
    assert_eq!(LogOps::next_expiry_secs(10), None);
}

#[test]
fn upstream_refusals_keep_canic_error_identity_and_the_existing_history() {
    let mut selected = None;
    append_entry(&mut selected, 10, "worker", None, ic_logging::Level::Error);
    let too_long = "x".repeat(LOG_LABEL_MAX_BYTES + 1);
    let error = append(
        &mut selected,
        ic_logging::EntryInput {
            created_at_secs: 11,
            level: ic_logging::Level::Info,
            source: &too_long,
            topic: None,
            message: "rejected",
        },
        2_000,
        16,
    )
    .unwrap_err();
    assert_eq!(error.code(), codes::CAPACITY_LIMIT);
    let error = append(
        &mut selected,
        ic_logging::EntryInput {
            created_at_secs: 9,
            level: ic_logging::Level::Info,
            source: "worker",
            topic: None,
            message: "rejected",
        },
        2_000,
        16,
    )
    .unwrap_err();
    assert_eq!(error.code(), codes::TIME_ORDERING);
    let runtime = selected.unwrap();
    let page = project_page(
        &runtime.log,
        ic_logging::Filter::default(),
        PageRequest {
            limit: 10,
            offset: 0,
        },
    );
    assert_eq!(page.total, 1);
    assert_eq!(page.entries[0].created_at, 10);
}
