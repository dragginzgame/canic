use super::timing::{InstallTimingLabel, InstallTimingSummary};
use crate::table::{ColumnAlign, render_table};
use std::{path::Path, time::Duration};

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
        timing_row(InstallTimingLabel::FUND_ROOT, timings.fund_root),
        timing_row(
            InstallTimingLabel::STAGE_RELEASE_SET,
            timings.stage_release_set,
        ),
        timing_row(
            InstallTimingLabel::RESUME_BOOTSTRAP,
            timings.resume_bootstrap,
        ),
        timing_row(InstallTimingLabel::WAIT_READY, timings.wait_ready),
        timing_row(
            InstallTimingLabel::FINALIZE_ROOT_FUNDING,
            timings.finalize_root_funding,
        ),
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

pub(super) fn print_install_result_summary(
    environment: &str,
    deployment: &str,
    fleet_template: &str,
    state_path: &Path,
) {
    println!("Install result:");
    println!("{:<14} success", "status");
    println!("{:<14} {}", "deployment", deployment);
    println!("{:<14} {}", "fleet_template", fleet_template);
    println!("{:<14} {}", "install_state", state_path.display());
    println!(
        "{:<14} canic list {} --environment {}",
        "smoke_check", deployment, environment
    );
}
