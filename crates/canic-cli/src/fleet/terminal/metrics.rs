//! Diagnostic projection only; supplier counters are never summed into operation counts.
use canic_host::Fleet;
use ic_host_text::{Alignment, escape, plain_table};
use std::fmt::Write as _;

pub(super) fn render(snapshot: &Fleet) -> String {
    let mut output = String::new();
    for report in &snapshot.metrics {
        let name = super::roles::name(snapshot, &report.network_id, &report.canister)
            .unwrap_or_else(|| report.canister.clone());
        writeln!(output, "\nTimer metrics (uncertified): {}", escape(&name)).expect("String write");
        writeln!(output, "  Network identity: {}", escape(&report.network_id))
            .expect("String write");
        let Some(timers) = report.reported.as_ref().and_then(|s| s.timers.as_ref()) else {
            writeln!(
                output,
                "  UNKNOWN: {}",
                escape(
                    report
                        .observation_error
                        .as_deref()
                        .unwrap_or("timer metrics unavailable")
                )
            )
            .expect("String write");
            continue;
        };
        writeln!(
            output,
            "  Runtime started at version {}, {} Unix ns; sampled {} Unix ns.",
            timers.started_at_version,
            timers.started_at_ns,
            report
                .reported
                .as_ref()
                .expect("reported timers")
                .observed_at_ns
        )
        .expect("String write");
        let rows = super::super::metrics::rows(report).expect("reported timers");
        output.push_str(&plain_table(
            &["COUNTER", "TOTAL", "QUALITY"],
            &rows,
            &[Alignment::Left, Alignment::Right, Alignment::Left],
        ));
        output.push_str(
            "\n  Volatile since initialization; counters do not prove operation outcomes.\n",
        );
    }
    output
}
