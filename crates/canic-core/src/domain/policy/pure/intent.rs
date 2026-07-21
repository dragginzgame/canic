//! Module: domain::policy::pure::intent
//!
//! Responsibility: decide application receipt replay-window admission.
//! Does not own: clocks, receipt lookup, storage, or downstream authorization.
//! Boundary: workflow captures time once; ops applies this decision only after exact lookup misses.

use crate::model::intent::{
    MAX_RECEIPT_BACKED_INTENT_REPLAY_WINDOW_NS, ReceiptReplayWindowDecision,
};

#[must_use]
pub const fn decide_receipt_replay_window(
    now_ns: u64,
    replay_deadline_ns: u64,
) -> ReceiptReplayWindowDecision {
    if now_ns >= replay_deadline_ns {
        return ReceiptReplayWindowDecision::Closed;
    }

    let remaining_ns = replay_deadline_ns - now_ns;
    if remaining_ns > MAX_RECEIPT_BACKED_INTENT_REPLAY_WINDOW_NS {
        ReceiptReplayWindowDecision::TooLong { remaining_ns }
    } else {
        ReceiptReplayWindowDecision::Open
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_replay_window_boundaries_are_exact() {
        let now = 100;

        assert_eq!(
            decide_receipt_replay_window(now, now),
            ReceiptReplayWindowDecision::Closed
        );
        assert_eq!(
            decide_receipt_replay_window(now, now - 1),
            ReceiptReplayWindowDecision::Closed
        );
        assert_eq!(
            decide_receipt_replay_window(now, now + MAX_RECEIPT_BACKED_INTENT_REPLAY_WINDOW_NS),
            ReceiptReplayWindowDecision::Open
        );
        assert_eq!(
            decide_receipt_replay_window(now, now + MAX_RECEIPT_BACKED_INTENT_REPLAY_WINDOW_NS + 1),
            ReceiptReplayWindowDecision::TooLong {
                remaining_ns: MAX_RECEIPT_BACKED_INTENT_REPLAY_WINDOW_NS + 1,
            }
        );
    }
}
