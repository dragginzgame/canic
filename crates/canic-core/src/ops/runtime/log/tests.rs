use super::*;

fn append_entry(
    selected: &mut Option<ic_logging::Log>,
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
    let log = selected.unwrap();
    let page = project_page(
        &log,
        ic_logging::Filter {
            source: Some("worker"),
            topic: ic_logging::TopicMatch::Exact(Topic::Cycles.log_label()),
            min_level: Some(upstream_level(Level::Warn)),
        },
        LogPageRequest {
            limit: 10,
            before: None,
        },
    )
    .unwrap();
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries.len(), 1);
    let row = &page.entries[0];
    assert_eq!(row.crate_name, "worker");
    assert_eq!(row.topic.as_deref(), Some("cycles"));
    assert_eq!(row.level, Level::Warn);
    assert_eq!(row.created_at, 3);
    assert!(row.message.len() <= 16);
}

#[test]
fn cursor_scans_bound_work_and_continue_after_empty_filtered_pages() {
    let mut selected = None;
    for time in 0..2_000 {
        let source = if time < 300 { "worker" } else { "other" };
        append_entry(&mut selected, time, source, None, ic_logging::Level::Info);
    }
    let log = selected.unwrap();
    let filter = ic_logging::Filter {
        source: Some("worker"),
        ..ic_logging::Filter::default()
    };
    let first = project_page(
        &log,
        filter,
        LogPageRequest {
            limit: 2,
            before: None,
        },
    )
    .unwrap();
    assert!(first.entries.is_empty());
    assert_eq!(first.scanned, LOG_SCAN_MAX);
    assert!(first.next_before.is_some());
    let second = project_page(
        &log,
        filter,
        LogPageRequest {
            limit: 2,
            before: first.next_before,
        },
    )
    .unwrap();
    assert!(second.scanned <= LOG_SCAN_MAX);
    assert_eq!(
        second
            .entries
            .iter()
            .map(|entry| entry.created_at)
            .collect::<Vec<_>>(),
        [299, 298]
    );
    let third = project_page(
        &log,
        filter,
        LogPageRequest {
            limit: 2,
            before: second.next_before,
        },
    )
    .unwrap();
    assert_eq!(
        third
            .entries
            .iter()
            .map(|entry| entry.created_at)
            .collect::<Vec<_>>(),
        [297, 296]
    );
}

#[test]
fn canic_reply_limit_is_independent_of_the_upstream_scan_budget() {
    let mut selected = None;
    for time in 0..1_100 {
        append_entry(&mut selected, time, "worker", None, ic_logging::Level::Info);
    }
    let log = selected.unwrap();
    let page = project_page(
        &log,
        ic_logging::Filter::default(),
        LogPageRequest {
            limit: u16::MAX,
            before: None,
        },
    )
    .unwrap();
    assert_eq!(page.scanned as usize, LOG_PAGE_MAX_LIMIT);
    assert_eq!(page.next_before, Some(100));
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
        LogPageRequest {
            limit: 10,
            before: None,
        },
    )
    .unwrap();
    assert_eq!(page.entries.len(), 0);
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
    let log = selected.unwrap();
    let page = project_page(
        &log,
        ic_logging::Filter::default(),
        LogPageRequest {
            limit: 10,
            before: None,
        },
    )
    .unwrap();
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].created_at, 10);
}

#[test]
fn retained_text_ceiling_preserves_configured_capacity_at_all_field_limits() {
    let mut selected = None;
    let label = "x".repeat(LOG_LABEL_MAX_BYTES);
    for sequence in 0..3 {
        assert_eq!(
            append(
                &mut selected,
                ic_logging::EntryInput {
                    created_at_secs: sequence,
                    level: ic_logging::Level::Info,
                    source: &label,
                    topic: Some(&label),
                    message: "abcdefghijklmnop",
                },
                2,
                16,
            )
            .unwrap(),
            Some(sequence),
        );
    }
    let log = selected.unwrap();
    let page = project_page(
        &log,
        ic_logging::Filter::default(),
        LogPageRequest {
            limit: 10,
            before: None,
        },
    )
    .unwrap();
    assert_eq!(page.entries.len(), 2);
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.created_at)
            .collect::<Vec<_>>(),
        [2, 1]
    );
    assert_eq!(log.retained_text_bytes(), log.limits().max_text_bytes);
}

#[test]
fn changed_host_limits_start_a_new_log_window() {
    for (max_entries, max_message_bytes) in [(2_001, 16), (2_000, 8)] {
        let mut selected = None;
        append_entry(&mut selected, 1, "old", None, ic_logging::Level::Info);
        append_entry(&mut selected, 2, "old", None, ic_logging::Level::Info);
        append(
            &mut selected,
            ic_logging::EntryInput {
                created_at_secs: 3,
                level: ic_logging::Level::Info,
                source: "new",
                topic: None,
                message: "new diagnostic window",
            },
            max_entries,
            max_message_bytes,
        )
        .unwrap();
        let page = project_page(
            selected.as_ref().unwrap(),
            ic_logging::Filter::default(),
            LogPageRequest {
                before: None,
                limit: 10,
            },
        )
        .unwrap();
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].crate_name, "new");
        assert!(page.entries[0].message.len() <= max_message_bytes as usize);
    }
}

#[test]
fn reply_text_budget_preserves_every_row_across_cursor_pages() {
    let mut selected = None;
    let message = "🦀".repeat(16_384);
    for time in 0..32 {
        append(
            &mut selected,
            ic_logging::EntryInput {
                created_at_secs: time,
                level: ic_logging::Level::Info,
                source: "worker",
                topic: None,
                message: &message,
            },
            64,
            65_536,
        )
        .unwrap();
    }
    let log = selected.unwrap();
    let mut before = None;
    let mut seen = Vec::new();
    loop {
        let page = project_page(
            &log,
            ic_logging::Filter::default(),
            LogPageRequest {
                before,
                limit: 1000,
            },
        )
        .unwrap();
        assert!(page.scanned <= LOG_SCAN_MAX);
        let reply: Result<&LogPage, canic_contracts::dto::error::Error> = Ok(&page);
        let bytes = ic_wire::native::encode_response(
            canic_contracts::protocol::protected_logs::METHOD_ID,
            &reply,
            canic_contracts::protocol::protected_logs::RESPONSE_LIMITS,
        )
        .unwrap();
        assert!(bytes.len() <= canic_contracts::protocol::protected_logs::RESPONSE_LIMITS.bytes);
        seen.extend(page.entries.iter().map(|entry| entry.created_at));
        match page.next_before {
            Some(next) => {
                assert!(before.is_none_or(|previous| next < previous));
                before = Some(next);
            }
            None => break,
        }
    }
    assert_eq!(seen, (0..32).rev().collect::<Vec<_>>());
}

#[test]
fn oversized_row_and_zero_limit_have_typed_refusals() {
    let mut selected = None;
    let message = "x".repeat(LOG_PAGE_TEXT_MAX_BYTES + 1);
    append(
        &mut selected,
        ic_logging::EntryInput {
            created_at_secs: 1,
            level: ic_logging::Level::Info,
            source: "worker",
            topic: None,
            message: &message,
        },
        2,
        u32::try_from(message.len()).unwrap(),
    )
    .unwrap();
    let error = project_page(
        selected.as_ref().unwrap(),
        ic_logging::Filter::default(),
        LogPageRequest {
            before: None,
            limit: 1,
        },
    )
    .unwrap_err();
    assert_eq!(error.code(), codes::CAPACITY_LIMIT);
    LogOps::reset_for_tests();
    let error = LogOps::page_filtered(
        None,
        None,
        None,
        LogPageRequest {
            before: None,
            limit: 0,
        },
    )
    .unwrap_err();
    assert_eq!(error.code(), InternalError::invalid_input().code());
}
