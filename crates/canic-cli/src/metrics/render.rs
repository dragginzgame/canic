use crate::{
    metrics::{
        MetricsCommandError,
        model::{MetricEntry, MetricValue, MetricsKind, MetricsReport},
        options::MetricsOptions,
    },
    output,
};
use canic_host::table::{ColumnAlign, render_table};

const DEFAULT_LABEL_MAX_CHARS: usize = 56;

pub(super) fn write_metrics_report(
    options: &MetricsOptions,
    report: &MetricsReport,
) -> Result<(), MetricsCommandError> {
    if options.json {
        return output::write_pretty_json::<_, MetricsCommandError>(options.out.as_deref(), report);
    }

    output::write_text::<MetricsCommandError>(
        options.out.as_deref(),
        &render_metrics_report(report, options.verbose),
    )
}

fn render_metrics_report(report: &MetricsReport, verbose: bool) -> String {
    [
        format!(
            "Deployment: {} (environment {}, metrics {})",
            report.deployment,
            report.environment,
            metrics_kind_label(report.kind)
        ),
        String::new(),
        if verbose {
            render_verbose_metrics_table(report)
        } else {
            render_default_metrics_table(report)
        },
    ]
    .join("\n")
}

fn render_default_metrics_table(report: &MetricsReport) -> String {
    let mut rows = Vec::new();
    for canister in &report.canisters {
        if canister.entries.is_empty() {
            rows.push([
                canister.role.clone(),
                canister.status.label().to_string(),
                "-".to_string(),
                canister.error.clone().unwrap_or_else(|| "-".to_string()),
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
            ]);
            continue;
        }

        for entry in &canister.entries {
            let value = metric_value_columns(&entry.value);
            rows.push([
                canister.role.clone(),
                canister.status.label().to_string(),
                metric_family_label(entry),
                metric_detail_label(entry, Some(DEFAULT_LABEL_MAX_CHARS)),
                value.count,
                value.average_per_count,
                value.amount,
            ]);
        }
    }

    render_table(
        &[
            "ROLE", "STATUS", "METRIC", "LABELS", "COUNT", "AVG/CALL", "AMOUNT",
        ],
        &rows,
        &[
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Right,
            ColumnAlign::Right,
            ColumnAlign::Right,
        ],
    )
}

fn render_verbose_metrics_table(report: &MetricsReport) -> String {
    let mut rows = Vec::new();
    for canister in &report.canisters {
        if canister.entries.is_empty() {
            rows.push([
                canister.role.clone(),
                canister.canister_id.clone(),
                metrics_kind_label(report.kind).to_string(),
                canister.status.label().to_string(),
                "-".to_string(),
                canister.error.clone().unwrap_or_else(|| "-".to_string()),
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
            ]);
            continue;
        }

        for entry in &canister.entries {
            let value = metric_value_columns(&entry.value);
            rows.push([
                canister.role.clone(),
                canister.canister_id.clone(),
                metrics_kind_label(report.kind).to_string(),
                canister.status.label().to_string(),
                metric_family_label(entry),
                metric_detail_label(entry, None),
                entry.principal.clone().unwrap_or_else(|| "-".to_string()),
                value.count,
                value.average_per_count,
                value.total,
                value.amount,
            ]);
        }
    }

    render_table(
        &[
            "ROLE",
            "CANISTER_ID",
            "KIND",
            "STATUS",
            "METRIC",
            "LABELS",
            "PRINCIPAL",
            "COUNT",
            "AVG/CALL",
            "TOTAL",
            "AMOUNT",
        ],
        &rows,
        &[
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
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

struct MetricValueColumns {
    count: String,
    average_per_count: String,
    total: String,
    amount: String,
}

fn metric_value_columns(value: &MetricValue) -> MetricValueColumns {
    match value {
        MetricValue::Count { count } => MetricValueColumns {
            count: count.to_string(),
            average_per_count: "-".to_string(),
            total: "-".to_string(),
            amount: "-".to_string(),
        },
        MetricValue::CountAndU64 { count, value_u64 } => MetricValueColumns {
            count: count.to_string(),
            average_per_count: average_per_count(*value_u64, *count),
            total: value_u64.to_string(),
            amount: "-".to_string(),
        },
        MetricValue::U128 { value } => MetricValueColumns {
            count: "-".to_string(),
            average_per_count: "-".to_string(),
            total: "-".to_string(),
            amount: value.to_string(),
        },
    }
}

fn metric_family_label(entry: &MetricEntry) -> String {
    entry
        .labels
        .first()
        .cloned()
        .unwrap_or_else(|| "-".to_string())
}

fn metric_detail_label(entry: &MetricEntry, max_chars: Option<usize>) -> String {
    let label = if entry.labels.len() > 1 {
        entry.labels[1..].join("/")
    } else {
        "-".to_string()
    };
    match max_chars {
        Some(max_chars) => compact_cell(&label, max_chars),
        None => label,
    }
}

fn compact_cell(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let prefix_chars = max_chars.saturating_sub(3);
    format!(
        "{}...",
        value.chars().take(prefix_chars).collect::<String>()
    )
}

fn average_per_count(total: u64, count: u64) -> String {
    if count == 0 {
        return "-".to_string();
    }

    let whole = total / count;
    let remainder = total % count;
    if remainder == 0 {
        return whole.to_string();
    }

    let count = u128::from(count);
    let tenths = ((u128::from(remainder) * 10) + (count / 2)) / count;
    if tenths == 10 {
        (whole + 1).to_string()
    } else {
        format!("{whole}.{tenths}")
    }
}

const fn metrics_kind_label(kind: MetricsKind) -> &'static str {
    match kind {
        MetricsKind::Core => "core",
        MetricsKind::Placement => "placement",
        MetricsKind::Platform => "platform",
        MetricsKind::Runtime => "runtime",
        MetricsKind::Security => "security",
        MetricsKind::Storage => "storage",
    }
}

// -----------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::model::{MetricsCanisterReport, MetricsCanisterStatus};

    const CANISTER_ID: &str = "renrk-eyaaa-aaaaa-aaada-cai";
    const PRINCIPAL: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
    const LONG_LABEL: &str =
        "endpoint/update/canic_prepare_delegated_token_with_a_very_long_runtime_probe_label";

    fn report() -> MetricsReport {
        MetricsReport {
            deployment: "demo-local".to_string(),
            environment: "local".to_string(),
            kind: MetricsKind::Runtime,
            canisters: vec![MetricsCanisterReport {
                role: "app".to_string(),
                canister_id: CANISTER_ID.to_string(),
                status: MetricsCanisterStatus::Ok,
                entries: vec![
                    MetricEntry {
                        labels: ["perf", LONG_LABEL]
                            .into_iter()
                            .map(str::to_string)
                            .collect(),
                        principal: Some(PRINCIPAL.to_string()),
                        value: MetricValue::CountAndU64 {
                            count: 3,
                            value_u64: 15,
                        },
                    },
                    MetricEntry {
                        labels: ["intent", "reserve", "ok"]
                            .into_iter()
                            .map(str::to_string)
                            .collect(),
                        principal: None,
                        value: MetricValue::Count { count: 2 },
                    },
                    MetricEntry {
                        labels: ["cycles_funding", "attached"]
                            .into_iter()
                            .map(str::to_string)
                            .collect(),
                        principal: None,
                        value: MetricValue::U128 { value: 1_000 },
                    },
                ],
                error: None,
            }],
        }
    }

    #[test]
    fn default_metrics_table_splits_count_and_average_without_verbose_columns() {
        let output = render_metrics_report(&report(), false);

        assert!(output.contains("COUNT"));
        assert!(output.contains("AVG/CALL"));
        assert!(output.contains("AMOUNT"));
        assert!(output.contains("perf"));
        assert!(output.contains('3'));
        assert!(output.contains('5'));
        assert!(output.contains("..."));
        assert!(!output.contains("CANISTER_ID"));
        assert!(!output.contains("PRINCIPAL"));
        assert!(!output.contains("TOTAL"));
        assert!(!output.contains(CANISTER_ID));
        assert!(!output.contains(PRINCIPAL));
        assert!(!output.contains(LONG_LABEL));
    }

    #[test]
    fn verbose_metrics_table_keeps_ids_principals_and_raw_totals() {
        let output = render_metrics_report(&report(), true);

        assert!(output.contains("CANISTER_ID"));
        assert!(output.contains("PRINCIPAL"));
        assert!(output.contains("TOTAL"));
        assert!(output.contains(CANISTER_ID));
        assert!(output.contains(PRINCIPAL));
        assert!(output.contains(LONG_LABEL));
        assert!(output.contains("15"));
    }

    #[test]
    fn averages_count_and_total_with_one_decimal_place_when_needed() {
        assert_eq!(average_per_count(15, 3), "5");
        assert_eq!(average_per_count(10, 4), "2.5");
        assert_eq!(average_per_count(1, 0), "-");
    }
}
