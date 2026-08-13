use super::timing::{InstallTimingLabel, InstallTimingSummary};
use crate::table::{ColumnAlign, render_bordered_table};
use std::time::Duration;

pub(super) use crate::terminal::{TerminalActivity, TerminalStyle};

pub(super) fn print_install_timing_summary(timings: &InstallTimingSummary, total: Duration) {
    TerminalStyle::detected().print_section("Install complete", "timing summary");
    println!("{}", render_install_timing_summary(timings, total));
}

pub(super) fn render_install_timing_summary(
    timings: &InstallTimingSummary,
    total: Duration,
) -> String {
    let phase_rows = [
        (InstallTimingLabel::PREFLIGHT, timings.preflight),
        (
            InstallTimingLabel::BUILD_CONFIGURED,
            timings.build_configured,
        ),
        (
            InstallTimingLabel::BUILD_INFRASTRUCTURE,
            timings.build_infrastructure,
        ),
        (
            InstallTimingLabel::MATERIALIZE_ARTIFACTS,
            timings.materialize_artifacts,
        ),
        (InstallTimingLabel::REUSE_ARTIFACTS, timings.reuse_artifacts),
        (InstallTimingLabel::POST_BUILD_GATE, timings.post_build_gate),
        (InstallTimingLabel::EMIT_MANIFEST, timings.emit_manifest),
        (InstallTimingLabel::ACTIVATE_FLEET, timings.activate_fleet),
    ]
    .into_iter()
    .map(|(label, duration)| (label, duration_centiseconds(duration)))
    .filter(|(_, centiseconds)| *centiseconds != 0)
    .collect::<Vec<_>>();
    let total_centiseconds = duration_centiseconds(total);
    let measured_centiseconds = phase_rows
        .iter()
        .map(|(_, centiseconds)| centiseconds)
        .sum::<u128>();
    let other_centiseconds = total_centiseconds.saturating_sub(measured_centiseconds);
    let rows = phase_rows
        .into_iter()
        .chain((other_centiseconds != 0).then_some((InstallTimingLabel::OTHER, other_centiseconds)))
        .chain([(InstallTimingLabel::TOTAL, total_centiseconds)])
        .map(|(label, centiseconds)| timing_row(label, centiseconds))
        .collect::<Vec<_>>();
    render_bordered_table(
        &["PHASE", "ELAPSED"],
        &rows,
        &[ColumnAlign::Left, ColumnAlign::Right],
    )
}

const fn duration_centiseconds(duration: Duration) -> u128 {
    duration.as_millis() / 10
}

fn timing_row(label: InstallTimingLabel, centiseconds: u128) -> [String; 2] {
    [
        label.as_str().to_string(),
        format!("{}.{:02}s", centiseconds / 100, centiseconds % 100),
    ]
}
