//! Check the operator page at the supplier's retention and Wire boundaries.

use super::*;

#[test]
fn retained_pages_fit_wire_and_never_repeat_an_entry() {
    let mut log = Log::new(LOG_LIMITS);
    let message = "x".repeat(200);
    for i in 0..150 {
        log.append(EntryInput {
            created_at_secs: i,
            level: Level::Info,
            source: "management",
            topic: None,
            message: &message,
        })
        .unwrap();
    }
    LOG.with_borrow_mut(|slot| *slot = Some((42, log)));
    let mut cursor = None;
    let mut timestamps = Vec::new();
    loop {
        let result = page(cursor);
        assert_eq!(result.started_at_version, 42);
        assert!(result.entries.len() <= 8);
        assert!(Logs::encode_reply(&result).is_ok());
        for row in result.entries {
            assert!(row.truncated);
            timestamps.push(row.created_at_secs);
        }
        let Some(next) = result.next_before else {
            break;
        };
        assert!(cursor.is_none_or(|previous| previous > next));
        cursor = Some(next);
    }
    assert_eq!(timestamps.first(), Some(&149));
    assert!(timestamps.len() > 8 && timestamps.len() < 128);
    assert!(timestamps.windows(2).all(|pair| pair[0] == pair[1] + 1));
}
