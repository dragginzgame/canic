use crate::{
    cycles::{
        CyclesCommandError,
        model::{CyclesCanisterReport, CyclesReport, CyclesTopupSummary},
        options::CyclesOptions,
    },
    output,
};
use canic_host::{
    format::{compact_duration, cycles_tc},
    table::{ColumnAlign, render_table},
};

pub(super) fn write_cycles_report(
    options: &CyclesOptions,
    report: &CyclesReport,
) -> Result<(), CyclesCommandError> {
    if options.json {
        return output::write_pretty_json::<_, CyclesCommandError>(options.out.as_ref(), report);
    }

    output::write_text::<CyclesCommandError>(
        options.out.as_ref(),
        &render_cycles_report(report, options.verbose),
    )
}

fn render_cycles_report(report: &CyclesReport, verbose: bool) -> String {
    [
        format!(
            "Fleet: {} (network {}, cycle balance since {})",
            report.fleet,
            report.network,
            compact_duration(report.since_seconds)
        ),
        String::new(),
        if verbose {
            render_verbose_cycles_table(report)
        } else {
            render_default_cycles_table(report)
        },
    ]
    .join("\n")
}

fn render_default_cycles_table(report: &CyclesReport) -> String {
    let rows = report
        .canisters
        .iter()
        .map(default_cycle_report_row)
        .collect::<Vec<_>>();
    render_table(
        &[
            "ROLE",
            "CANISTER_ID",
            "STATUS",
            "CURRENT",
            "BURN/H",
            "TOPUP/H",
            "NET/H",
        ],
        &rows,
        &[
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Right,
            ColumnAlign::Right,
            ColumnAlign::Right,
            ColumnAlign::Right,
        ],
    )
}

fn default_cycle_report_row(row: &CyclesCanisterReport) -> [String; 7] {
    [
        role_label(row),
        row.canister_id.clone(),
        row.status.clone(),
        row.latest_cycles.map_or_else(|| "-".to_string(), cycles_tc),
        row.burn_cycles_per_hour
            .map_or_else(|| "-".to_string(), format_unsigned_rate),
        row.topup_cycles_per_hour
            .map_or_else(|| "-".to_string(), format_unsigned_rate),
        row.rate_cycles_per_hour
            .map_or_else(|| "-".to_string(), format_signed_rate),
    ]
}

fn render_verbose_cycles_table(report: &CyclesReport) -> String {
    let rows = report
        .canisters
        .iter()
        .map(verbose_cycle_report_row)
        .collect::<Vec<_>>();
    render_table(
        &[
            "ROLE",
            "CANISTER_ID",
            "STATUS",
            "CURRENT",
            "BURN/H",
            "TOPUP/H",
            "NET/H",
            "HISTORY",
            "TOPUPS",
            "NET",
        ],
        &rows,
        &[
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Right,
            ColumnAlign::Right,
            ColumnAlign::Right,
            ColumnAlign::Right,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Right,
        ],
    )
}

fn verbose_cycle_report_row(row: &CyclesCanisterReport) -> [String; 10] {
    [
        role_label(row),
        row.canister_id.clone(),
        row.status.clone(),
        row.latest_cycles.map_or_else(|| "-".to_string(), cycles_tc),
        row.burn_cycles_per_hour
            .map_or_else(|| "-".to_string(), format_unsigned_rate),
        row.topup_cycles_per_hour
            .map_or_else(|| "-".to_string(), format_unsigned_rate),
        row.rate_cycles_per_hour
            .map_or_else(|| "-".to_string(), format_signed_rate),
        format_history(row),
        row.topups
            .as_ref()
            .map_or_else(|| "-".to_string(), format_topups),
        row.delta_cycles
            .map_or_else(|| "-".to_string(), format_signed_cycles),
    ]
}

fn role_label(row: &CyclesCanisterReport) -> String {
    format!("{}{}", row.tree_prefix, row.role)
}

fn format_history(row: &CyclesCanisterReport) -> String {
    if row.sample_count == 0 {
        return "-".to_string();
    }

    let coverage = row
        .coverage_seconds
        .map_or_else(|| "-".to_string(), compact_duration);
    if row.coverage_status == "covered" {
        format!("{} / {coverage}", row.sample_count)
    } else {
        format!("{} / {coverage} {}", row.sample_count, row.coverage_status)
    }
}

pub(super) fn format_topups(topups: &CyclesTopupSummary) -> String {
    let mut parts = Vec::new();
    if topups.request_ok > 0 {
        if topups.transferred_cycles > 0 {
            let transferred = cycles_tc(topups.transferred_cycles);
            if topups.request_ok == 1 {
                parts.push(transferred);
            } else {
                parts.push(format!("{transferred} ({})", topups.request_ok));
            }
        } else {
            parts.push(format!("{} ok", topups.request_ok));
        }
    }
    if topups.request_err > 0 {
        parts.push(format!("{} failed", topups.request_err));
    }
    if topups.request_scheduled > topups.request_ok.saturating_add(topups.request_err) {
        let pending = topups
            .request_scheduled
            .saturating_sub(topups.request_ok.saturating_add(topups.request_err));
        parts.push(format!("{pending} pending"));
    }
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(", ")
    }
}

fn format_signed_cycles(value: i128) -> String {
    if value < 0 {
        format!("-{}", cycles_tc(value.unsigned_abs()))
    } else {
        format!("+{}", cycles_tc(value.cast_unsigned()))
    }
}

fn format_signed_rate(value: i128) -> String {
    format!("{}/h", format_signed_cycles(value))
}

fn format_unsigned_rate(value: u128) -> String {
    format!("{}/h", cycles_tc(value))
}
