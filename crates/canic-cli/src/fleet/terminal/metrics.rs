//! Diagnostic projection only; supplier counters are never summed into operation counts.
use canic_host::Fleet;
use ic_host_text::{
    escape::terminal_text,
    table::{self, Alignment, Options},
};
use std::fmt::Write as _;

pub(super) fn render(snapshot: &Fleet) -> String {
    let mut output = String::new();
    for report in &snapshot.metrics {
        let name = super::roles::name(snapshot, &report.network_id, &report.canister)
            .unwrap_or_else(|| report.canister.clone());
        writeln!(
            output,
            "\nTimer metrics (uncertified): {}",
            terminal_text(&name)
        )
        .expect("String write");
        writeln!(
            output,
            "  Network identity: {}",
            terminal_text(&report.network_id)
        )
        .expect("String write");
        let Some(timers) = report.reported.as_ref().and_then(|s| s.timers.as_ref()) else {
            writeln!(
                output,
                "  UNKNOWN: {}",
                terminal_text(
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
        let rows = [
            ("Accepted", timers.accepted),
            ("Completed", timers.completed),
            ("Abandoned", timers.abandoned),
            ("Expected failures", timers.expected_failures),
            ("Deferred submissions", timers.deferred_submissions),
            ("Provider faults", timers.provider_faults),
        ]
        .map(|(name, value)| {
            [
                name.to_owned(),
                value.total.to_string(),
                if value.overflowed { "clipped" } else { "exact" }.to_owned(),
            ]
        });
        output.push_str(&table::render(
            &["COUNTER", "TOTAL", "QUALITY"],
            &rows,
            &[Alignment::Left, Alignment::Right, Alignment::Left],
            Options {
                column_gap: 2,
                header_separator: true,
            },
        ));
        output.push_str(
            "\n  Volatile since initialization; counters do not prove operation outcomes.\n",
        );
    }
    output
}
