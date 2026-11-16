//! Shared counter presentation and a collapsed, read-only HTML projection.
use canic_host::{Fleet, FleetMetrics};
use std::fmt::Write as _;

/// Both views retain supplier totals and quality without computing rates or effect counts.
pub(super) fn rows(report: &FleetMetrics) -> Option<[[String; 3]; 6]> {
    let timers = report.reported.as_ref()?.timers.as_ref()?;
    Some(
        [
            ("Accepted", timers.accepted),
            ("Completed", timers.completed),
            ("Abandoned", timers.abandoned),
            ("Expected failures", timers.expected_failures),
            ("Deferred submissions", timers.deferred_submissions),
            ("Provider faults", timers.provider_faults),
        ]
        .map(|(label, value)| {
            [
                label.to_owned(),
                value.total.to_string(),
                if value.overflowed { "clipped" } else { "exact" }.to_owned(),
            ]
        }),
    )
}

pub(super) fn render(snapshot: &Fleet) -> String {
    if snapshot.metrics.is_empty() {
        return String::new();
    }
    let mut html = String::from(
        "<details class=metrics><summary>Timer metrics · uncertified diagnostics</summary><p>Volatile since initialization; counters do not prove operation outcomes. An owner upgrade resets these counters. Missing observations are unknown, never zero.</p><div class=grid>",
    );
    for report in &snapshot.metrics {
        write!(html, "<article><h3>{}</h3><details><summary>Network trust binding</summary><code>{}</code></details>",
            super::escape(&report.canister), super::escape(&report.network_id)).expect("String write");
        if let Some(counters) = rows(report) {
            let observed = report.reported.as_ref().expect("counter report");
            let timers = observed.timers.as_ref().expect("timer counters");
            write!(html, "<p class=muted>Runtime started at version {}, <time data-ns=\"{}\">{} Unix seconds</time>. Sampled <time data-ns=\"{}\">{} Unix seconds</time>.</p><table class=metrics-table><thead><tr><th scope=col>Counter</th><th scope=col>Total</th><th scope=col>Quality</th></tr></thead><tbody>",
                timers.started_at_version, timers.started_at_ns, timers.started_at_ns / 1_000_000_000,
                observed.observed_at_ns, observed.observed_at_ns / 1_000_000_000).expect("String write");
            for [label, total, quality] in counters {
                write!(
                    html,
                    "<tr><th scope=row>{}</th><td><code>{}</code></td><td>{}</td></tr>",
                    super::escape(&label),
                    super::escape(&total),
                    super::escape(&quality)
                )
                .expect("String write");
            }
            html.push_str("</tbody></table>");
        } else {
            write!(
                html,
                "<p class=warning><strong>UNKNOWN</strong>: {}</p>",
                super::escape(
                    report
                        .observation_error
                        .as_deref()
                        .unwrap_or("timer metrics unavailable")
                )
            )
            .expect("String write");
        }
        html.push_str("</article>");
    }
    html.push_str("</div></details>");
    html
}
