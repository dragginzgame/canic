//! Module: support::build_cache
//!
//! Responsibility: summarize one complete-build cache decision for the operator.
//! Does not own: cache admission, environment binding or diagnostic attribution.
//! Boundary: detailed host explanations are presentation only and never authorize reuse.

/// Render a shared cache result, retaining the complete explanation only on request.
#[must_use]
pub fn cache_report(count: usize, miss_reason: Option<&str>, verbose: bool) -> String {
    let noun = if count == 1 { "artifact" } else { "artifacts" };
    let Some(reason) = miss_reason else {
        return format!("Cache: reusing {count} {noun} (verified complete build)");
    };
    // Host causes precede parenthesized attribution and comparison details.
    // This shortening is display-only; verbose output retains the original explanation.
    let summary = reason.split_once(" (").map_or(reason, |(cause, _)| cause);
    let mut report = format!("Cache: rebuilding {count} {noun} - {summary}");
    if summary != reason {
        if verbose {
            report.push_str("\nCache details: ");
            report.push_str(reason);
        } else {
            report.push_str("\nCache details: use --verbose");
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_miss_keeps_all_causes_and_emits_key_details_only_once_when_requested() {
        let reason = "source/dependency inputs changed; toolchain/configuration inputs changed; environment changed (added/removed keys, up to 8: TERM; changed-value keys, up to 8: PATH) (compared with last recorded successful build)";
        let concise = cache_report(8, Some(reason), false);
        assert!(concise.contains("rebuilding 8 artifacts"));
        assert!(concise.contains("source/dependency inputs changed"));
        assert!(concise.contains("toolchain/configuration inputs changed"));
        assert!(concise.contains("environment changed"));
        assert!(!concise.contains("TERM"));
        assert!(!concise.contains("PATH"));
        assert!(concise.contains("--verbose"));
        let detailed = cache_report(8, Some(reason), true);
        assert_eq!(detailed.matches(reason).count(), 1);
        assert!(!detailed.contains('\x1b'));
    }

    #[test]
    fn verified_hits_and_unattributed_or_rejected_misses_remain_distinct() {
        assert!(cache_report(8, None, true).contains("reusing 8 artifacts"));
        for reason in [
            "input comparison unavailable",
            "retained output/evidence rejected",
        ] {
            let report = cache_report(1, Some(reason), false);
            assert!(report.contains("rebuilding 1 artifact -"));
            assert!(report.ends_with(reason));
        }
    }
}
