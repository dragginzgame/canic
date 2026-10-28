//! Render cycle reports with shared columns and optional history detail.

use crate::{
    cycles::{
        CyclesCommandError,
        model::{CyclesCanisterReport, CyclesCoverageStatus, CyclesReport, CyclesTopupSummary},
        options::CyclesOptions,
    },
    output,
};
use canic_host::format::{compact_duration, cycles_tc};
use ic_host_text::table::{self, Alignment, Options};

pub(super) fn write_cycles_report(
    options: &CyclesOptions,
    report: &CyclesReport,
) -> Result<(), CyclesCommandError> {
    if options.json {
        return output::write_pretty_json::<_, CyclesCommandError>(options.out.as_deref(), report);
    }

    output::write_text::<CyclesCommandError>(
        options.out.as_deref(),
        &render_cycles_report(report, options.verbose),
    )
}

fn render_cycles_report(report: &CyclesReport, verbose: bool) -> String {
    [
        format!(
            "Fleet: {} (environment {}, cycle balance since {})",
            report.fleet,
            report.environment,
            compact_duration(report.since_seconds)
        ),
        String::new(),
        if verbose {
            render_cycles_table(report, verbose_cycle_report_row)
        } else {
            render_cycles_table(report, default_cycle_report_row)
        },
    ]
    .join("\n")
}

const CYCLE_COLUMNS: [(&str, Alignment); 10] = [
    ("ROLE", Alignment::Left),
    ("CANISTER_ID", Alignment::Left),
    ("STATUS", Alignment::Left),
    ("CURRENT", Alignment::Right),
    ("BURN/H", Alignment::Right),
    ("TOPUP/H", Alignment::Right),
    ("NET/H", Alignment::Right),
    ("HISTORY", Alignment::Left),
    ("TOPUPS", Alignment::Left),
    ("NET", Alignment::Right),
];

// Both fixed row shapes select their corresponding prefix of the same columns.
fn render_cycles_table<const N: usize>(
    report: &CyclesReport,
    row: fn(&CyclesCanisterReport) -> [String; N],
) -> String {
    let rows = report.canisters.iter().map(row).collect::<Vec<_>>();
    table::render(
        &std::array::from_fn(|index| CYCLE_COLUMNS[index].0),
        &rows,
        &std::array::from_fn(|index| CYCLE_COLUMNS[index].1),
        Options {
            column_gap: 3,
            header_separator: true,
        },
    )
}

fn default_cycle_report_row(row: &CyclesCanisterReport) -> [String; 7] {
    [
        role_label(row),
        row.canister_id.clone(),
        row.status.label().to_string(),
        row.latest_cycles.map_or_else(|| "-".to_string(), cycles_tc),
        row.burn_cycles_per_hour
            .map_or_else(|| "-".to_string(), format_unsigned_rate),
        row.topup_cycles_per_hour
            .map_or_else(|| "-".to_string(), format_unsigned_rate),
        row.rate_cycles_per_hour
            .map_or_else(|| "-".to_string(), format_signed_rate),
    ]
}

fn verbose_cycle_report_row(row: &CyclesCanisterReport) -> [String; 10] {
    let [role, canister_id, status, current, burn, topup, net] = default_cycle_report_row(row);
    [
        role,
        canister_id,
        status,
        current,
        burn,
        topup,
        net,
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
    if row.coverage_status == CyclesCoverageStatus::Covered {
        format!("{} / {coverage}", row.sample_count)
    } else {
        format!(
            "{} / {coverage} {}",
            row.sample_count,
            row.coverage_status.label()
        )
    }
}

fn format_topups(topups: &CyclesTopupSummary) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure structured top-up summaries become compact top-up context for cycles output.
    #[test]
    fn formats_topups() {
        let summary = CyclesTopupSummary {
            request_scheduled: 0,
            request_ok: 2,
            request_err: 0,
            transferred_cycles: 8_000_000_000_000,
        };

        assert_eq!(format_topups(&summary), "8.00 TC (2)");
    }
}
