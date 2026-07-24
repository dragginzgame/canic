use super::timing::{InstallTimingLabel, InstallTimingSummary};
use crate::table::{ColumnAlign, render_table};
use std::time::Duration;

pub(super) fn print_install_timing_summary(timings: &InstallTimingSummary, total: Duration) {
    println!("Install timing summary:");
    println!("{}", render_install_timing_summary(timings, total));
}

pub(super) fn render_install_timing_summary(
    timings: &InstallTimingSummary,
    total: Duration,
) -> String {
    let rows = [
        timing_row(
            InstallTimingLabel::CREATE_CANISTERS,
            timings.create_canisters,
        ),
        timing_row(InstallTimingLabel::BUILD_ALL, timings.build_all),
        timing_row(InstallTimingLabel::EMIT_MANIFEST, timings.emit_manifest),
        timing_row(InstallTimingLabel::INSTALL_ROOT, timings.install_root),
        timing_row(InstallTimingLabel::TOTAL, total),
    ];
    render_table(
        &["PHASE", "ELAPSED"],
        &rows,
        &[ColumnAlign::Left, ColumnAlign::Right],
    )
}

fn timing_row(label: InstallTimingLabel, duration: Duration) -> [String; 2] {
    [
        label.as_str().to_string(),
        format!("{:.2}s", duration.as_secs_f64()),
    ]
}
