//! Check the operator page at the supplier's retention and Wire boundaries.

use super::*;

#[test]
fn mixed_entries_share_one_retained_byte_budget() {
    let mut log = Log::with_retained_byte_limit(LOG_LIMITS);
    let long = "x".repeat(200);
    let rows = [
        ("", ""),
        ("management", "ok"),
        ("management", long.as_str()),
    ];
    let mut expected = std::collections::VecDeque::new();
    let mut bytes = 0;
    for i in 0..10_000 {
        let (source, message) = rows[i % rows.len()];
        let truncated = message.len() > LOG_LIMITS.max_message_bytes;
        let charge = ic_logging::RETAINED_ENTRY_OVERHEAD
            + source.len()
            + message.len().min(LOG_LIMITS.max_message_bytes);
        log.append(EntryInput {
            created_at_secs: i as u64,
            level: Level::Info,
            source,
            topic: None,
            message,
        })
        .unwrap();
        expected.push_back((i as u64, truncated, charge));
        bytes += charge;
        while bytes > DEFAULT_LOG_BYTES {
            bytes -= expected.pop_front().unwrap().2;
        }
    }
    LOG.with(|slot| assert!(slot.set((42, RefCell::new(log))).is_ok()));
    assert_eq!(retained_bytes(), bytes);
    let mut cursor = None;
    loop {
        let result = page(cursor);
        assert_eq!(result.started_at_version, 42);
        assert!(result.entries.len() <= 8);
        assert!(Logs::encode_reply(&result).is_ok());
        for row in result.entries {
            let (timestamp, truncated, _) = expected.pop_back().unwrap();
            assert_eq!((row.created_at_secs, row.truncated), (timestamp, truncated));
        }
        let Some(next) = result.next_before else {
            break;
        };
        assert!(cursor.is_none_or(|previous| previous > next));
        cursor = Some(next);
    }
    assert!(expected.is_empty());
}
