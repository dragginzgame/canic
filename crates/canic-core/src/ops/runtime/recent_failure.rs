//! Module: ops::runtime::recent_failure
//!
//! Responsibility: keep a heap-only, bounded, redacted recent-failure ring.
//! Does not own: durable logging, stable schemas, metrics, or repair behavior.
//! Boundary: exposes metadata-only failure summaries for guarded runtime status.

use crate::{domain::runtime::FailureSeverity, dto::runtime::RecentFailure};
use std::{cell::RefCell, collections::VecDeque};

const MAX_RECENT_FAILURES: usize = 16;
const MAX_SUBSYSTEM_BYTES: usize = 64;
const MAX_CODE_BYTES: usize = 96;
const MAX_SUMMARY_BYTES: usize = 256;
const MAX_CORRELATION_ID_BYTES: usize = 96;

thread_local! {
    static RECENT_FAILURES: RefCell<VecDeque<RecentFailure>> = const {
        RefCell::new(VecDeque::new())
    };
}

///
/// RecentFailureInput
///
/// Caller-owned failure metadata before runtime privacy bounds are applied.
///

pub struct RecentFailureInput {
    pub occurred_at_ns: u64,
    pub subsystem: String,
    pub code: String,
    pub severity: FailureSeverity,
    pub summary: String,
    pub correlation_id: Option<String>,
}

///
/// RecentFailureOps
///
/// Heap-only bounded recent-failure ring used by guarded runtime introspection.
///

pub struct RecentFailureOps;

impl RecentFailureOps {
    /// Record one redacted recent-failure summary.
    ///
    /// This is intentionally heap-only and best-effort. It does not write
    /// stable state and is cleared by upgrade/reinstall.
    pub fn record(input: RecentFailureInput) {
        let failure = project(input);

        RECENT_FAILURES.with_borrow_mut(|failures| {
            failures.push_front(failure);
            while failures.len() > MAX_RECENT_FAILURES {
                let _ = failures.pop_back();
            }
        });
    }

    /// Project one current failure ahead of the retained snapshot without mutation.
    #[must_use]
    pub fn snapshot_with(input: RecentFailureInput) -> Vec<RecentFailure> {
        let mut failures = Vec::with_capacity(MAX_RECENT_FAILURES);
        failures.push(project(input));
        failures.extend(Self::snapshot().into_iter().take(MAX_RECENT_FAILURES - 1));
        failures
    }

    #[must_use]
    pub fn snapshot() -> Vec<RecentFailure> {
        RECENT_FAILURES.with_borrow(|failures| failures.iter().cloned().collect())
    }

    #[cfg(test)]
    pub fn reset() {
        RECENT_FAILURES.with_borrow_mut(VecDeque::clear);
    }
}

fn project(input: RecentFailureInput) -> RecentFailure {
    let (subsystem, subsystem_redacted) = bounded_text(&input.subsystem, MAX_SUBSYSTEM_BYTES);
    let (code, code_redacted) = bounded_text(&input.code, MAX_CODE_BYTES);
    let (summary, summary_redacted) = bounded_text(&input.summary, MAX_SUMMARY_BYTES);
    let (correlation_id, correlation_redacted) = input
        .correlation_id
        .as_deref()
        .map(|value| bounded_text(value, MAX_CORRELATION_ID_BYTES))
        .map_or((None, false), |(value, redacted)| (Some(value), redacted));

    RecentFailure {
        occurred_at_ns: input.occurred_at_ns,
        subsystem,
        code,
        severity: input.severity,
        summary,
        correlation_id,
        redacted: subsystem_redacted || code_redacted || summary_redacted || correlation_redacted,
    }
}

fn bounded_text(value: &str, max_bytes: usize) -> (String, bool) {
    let mut output = String::new();
    let mut redacted = false;
    for ch in value.chars() {
        let ch = if ch.is_control() {
            redacted = true;
            ' '
        } else {
            ch
        };
        if output.len() + ch.len_utf8() > max_bytes {
            redacted = true;
            break;
        }
        output.push(ch);
    }
    (output, redacted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(index: u64) -> RecentFailureInput {
        RecentFailureInput {
            occurred_at_ns: index,
            subsystem: "runtime".to_string(),
            code: format!("failure_{index}"),
            severity: FailureSeverity::Warning,
            summary: format!("failure {index}"),
            correlation_id: None,
        }
    }

    #[test]
    fn recent_failures_are_bounded_and_newest_first() {
        RecentFailureOps::reset();

        for index in 0..20 {
            RecentFailureOps::record(input(index));
        }

        let failures = RecentFailureOps::snapshot();

        assert_eq!(failures.len(), MAX_RECENT_FAILURES);
        assert_eq!(failures[0].occurred_at_ns, 19);
        assert_eq!(failures[15].occurred_at_ns, 4);

        RecentFailureOps::reset();
    }

    #[test]
    fn recent_failure_summary_is_bounded_and_redacted() {
        RecentFailureOps::reset();
        RecentFailureOps::record(RecentFailureInput {
            occurred_at_ns: 1,
            subsystem: format!("auth\n{}", "s".repeat(MAX_SUBSYSTEM_BYTES + 10)),
            code: format!("token_failed\n{}", "c".repeat(MAX_CODE_BYTES + 10)),
            severity: FailureSeverity::Error,
            summary: format!("line\n{}", "x".repeat(MAX_SUMMARY_BYTES + 10)),
            correlation_id: Some("c".repeat(MAX_CORRELATION_ID_BYTES + 10)),
        });

        let failure = RecentFailureOps::snapshot()
            .into_iter()
            .next()
            .expect("failure");

        assert!(failure.redacted);
        assert!(failure.subsystem.len() <= MAX_SUBSYSTEM_BYTES);
        assert!(!failure.subsystem.contains('\n'));
        assert!(failure.code.len() <= MAX_CODE_BYTES);
        assert!(!failure.code.contains('\n'));
        assert!(failure.summary.len() <= MAX_SUMMARY_BYTES);
        assert!(!failure.summary.contains('\n'));
        assert!(
            failure
                .correlation_id
                .as_deref()
                .is_some_and(|value| value.len() <= MAX_CORRELATION_ID_BYTES)
        );

        RecentFailureOps::reset();
    }

    #[test]
    fn current_failure_projection_is_bounded_without_mutating_the_ring() {
        RecentFailureOps::reset();
        RecentFailureOps::record(input(1));

        let projected = RecentFailureOps::snapshot_with(input(2));

        assert_eq!(projected.len(), 2);
        assert_eq!(projected[0].occurred_at_ns, 2);
        assert_eq!(projected[1].occurred_at_ns, 1);
        assert_eq!(RecentFailureOps::snapshot().len(), 1);
        RecentFailureOps::reset();
    }
}
