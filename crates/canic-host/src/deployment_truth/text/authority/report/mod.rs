use super::super::super::*;
use super::super::{append_hard_failure_items, safety_status_label};
use super::shared::{
    append_authority_action_summary, append_blockers, append_next_actions,
    append_observation_gap_items,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AuthorityReportTextLabel(&'static str);

impl AuthorityReportTextLabel {
    const ALREADY_CORRECT: Self = Self("already_correct");
    const APPLY_READINESS: Self = Self("apply_readiness");
    const AUTHORITY_PROFILE_HASH: Self = Self("authority_profile_hash");
    const AUTOMATIC_ACTION_COUNT: Self = Self("automatic_action_count");
    const CAN_APPLY_AUTOMATICALLY: Self = Self("can_apply_automatically");
    const CHECK_ID: Self = Self("check_id");
    const COUNTS: Self = Self("counts");
    const HARD_FAILURES: Self = Self("hard_failures");
    const INVENTORY_ID: Self = Self("inventory_id");
    const MODE_DRY_RUN: Self = Self("mode: dry_run");
    const NOT_RECORDED: Self = Self("not recorded");
    const OBSERVATION_GAPS: Self = Self("observation_gaps");
    const PLAN_ID: Self = Self("plan_id");
    const REPORT_ID: Self = Self("report_id");
    const REQUIRES_EXTERNAL_ACTION: Self = Self("requires_external_action");
    const STATUS: Self = Self("status");
    const SUMMARY: Self = Self("summary");
    const TITLE: Self = Self("Authority reconciliation report");
    const UNKNOWN: Self = Self("unknown");
    const UNSAFE_BLOCKED: Self = Self("unsafe_blocked");

    #[must_use]
    const fn as_str(self) -> &'static str {
        self.0
    }
}

/// Render an authority report as read-only operator text.
#[must_use]
pub fn authority_report_text(report: &AuthorityReportV1) -> String {
    let mut lines = authority_report_header_lines(report);
    lines.push(String::new());
    lines.extend(authority_report_count_lines(report));
    lines.push(String::new());
    lines.extend(authority_report_apply_readiness_lines(report));
    append_blockers(&mut lines, report);
    append_next_actions(&mut lines, report);
    append_hard_failure_items(
        &mut lines,
        AuthorityReportTextLabel::HARD_FAILURES.as_str(),
        &report.hard_failures,
    );
    append_observation_gap_items(
        &mut lines,
        AuthorityReportTextLabel::OBSERVATION_GAPS.as_str(),
        &report.observation_gaps,
    );
    append_authority_action_summary(&mut lines, report);
    lines.join("\n")
}

fn authority_report_header_lines(report: &AuthorityReportV1) -> Vec<String> {
    vec![
        AuthorityReportTextLabel::TITLE.as_str().to_string(),
        AuthorityReportTextLabel::MODE_DRY_RUN.as_str().to_string(),
        format!(
            "{}: {}",
            AuthorityReportTextLabel::STATUS.as_str(),
            safety_status_label(report.status)
        ),
        format!(
            "{}: {}",
            AuthorityReportTextLabel::SUMMARY.as_str(),
            report.summary
        ),
        format!(
            "{}: {}",
            AuthorityReportTextLabel::REPORT_ID.as_str(),
            report.report_id
        ),
        format!(
            "{}: {}",
            AuthorityReportTextLabel::CHECK_ID.as_str(),
            report
                .check_id
                .as_deref()
                .unwrap_or(AuthorityReportTextLabel::NOT_RECORDED.as_str())
        ),
        format!(
            "{}: {}",
            AuthorityReportTextLabel::PLAN_ID.as_str(),
            report.reconciliation_plan_id
        ),
        format!(
            "{}: {}",
            AuthorityReportTextLabel::INVENTORY_ID.as_str(),
            report.inventory_id
        ),
        format!(
            "{}: {}",
            AuthorityReportTextLabel::AUTHORITY_PROFILE_HASH.as_str(),
            report
                .authority_profile_hash
                .as_deref()
                .unwrap_or(AuthorityReportTextLabel::NOT_RECORDED.as_str())
        ),
    ]
}

fn authority_report_count_lines(report: &AuthorityReportV1) -> Vec<String> {
    vec![
        format!("{}:", AuthorityReportTextLabel::COUNTS.as_str()),
        format!(
            "  {}: {}",
            AuthorityReportTextLabel::ALREADY_CORRECT.as_str(),
            report.counts.already_correct
        ),
        format!(
            "  {}: {}",
            AuthorityReportTextLabel::CAN_APPLY_AUTOMATICALLY.as_str(),
            report.counts.can_apply_automatically
        ),
        format!(
            "  {}: {}",
            AuthorityReportTextLabel::REQUIRES_EXTERNAL_ACTION.as_str(),
            report.counts.requires_external_action
        ),
        format!(
            "  {}: {}",
            AuthorityReportTextLabel::UNSAFE_BLOCKED.as_str(),
            report.counts.unsafe_blocked
        ),
        format!(
            "  {}: {}",
            AuthorityReportTextLabel::UNKNOWN.as_str(),
            report.counts.unknown
        ),
        format!(
            "  {}: {}",
            AuthorityReportTextLabel::HARD_FAILURES.as_str(),
            report.counts.hard_failures
        ),
    ]
}

fn authority_report_apply_readiness_lines(report: &AuthorityReportV1) -> Vec<String> {
    vec![
        format!("{}:", AuthorityReportTextLabel::APPLY_READINESS.as_str()),
        format!(
            "  {}: {}",
            AuthorityReportTextLabel::CAN_APPLY_AUTOMATICALLY.as_str(),
            report.apply_readiness.can_apply_automatically
        ),
        format!(
            "  {}: {}",
            AuthorityReportTextLabel::AUTOMATIC_ACTION_COUNT.as_str(),
            report.apply_readiness.automatic_action_count
        ),
    ]
}
