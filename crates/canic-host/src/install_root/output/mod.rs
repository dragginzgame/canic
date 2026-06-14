use super::timing::InstallTimingSummary;
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
        timing_row("create_canisters", timings.create_canisters),
        timing_row("build_all", timings.build_all),
        timing_row("emit_manifest", timings.emit_manifest),
        timing_row("install_root", timings.install_root),
        timing_row("fund_root", timings.fund_root),
        timing_row("stage_release_set", timings.stage_release_set),
        timing_row("resume_bootstrap", timings.resume_bootstrap),
        timing_row("wait_ready", timings.wait_ready),
        timing_row("finalize_root_funding", timings.finalize_root_funding),
        timing_row("total", total),
    ];
    render_table(
        &["PHASE", "ELAPSED"],
        &rows,
        &[ColumnAlign::Left, ColumnAlign::Right],
    )
}

fn timing_row(label: &str, duration: Duration) -> [String; 2] {
    [label.to_string(), format!("{:.2}s", duration.as_secs_f64())]
}

pub(super) fn print_install_result_summary(
    network: &str,
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
        "{:<14} canic list {} --network {}",
        "smoke_check", deployment, network
    );
}
