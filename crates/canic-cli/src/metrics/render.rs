use crate::{
    metrics::{
        MetricsCommandError,
        model::{MetricValue, MetricsReport},
        options::MetricsOptions,
    },
    output,
};
use canic_host::table::{ColumnAlign, render_table};

pub(super) fn write_metrics_report(
    options: &MetricsOptions,
    report: &MetricsReport,
) -> Result<(), MetricsCommandError> {
    if options.json {
        return output::write_pretty_json::<_, MetricsCommandError>(options.out.as_ref(), report);
    }

    output::write_text::<MetricsCommandError>(options.out.as_ref(), &render_metrics_report(report))
}

fn render_metrics_report(report: &MetricsReport) -> String {
    let mut rows = Vec::new();
    for canister in &report.canisters {
        if canister.entries.is_empty() {
            rows.push([
                canister.role.clone(),
                canister.canister_id.clone(),
                report.kind.as_str().to_string(),
                canister.status.clone(),
                canister.error.clone().unwrap_or_else(|| "-".to_string()),
                "-".to_string(),
                "-".to_string(),
            ]);
            continue;
        }

        for entry in &canister.entries {
            rows.push([
                canister.role.clone(),
                canister.canister_id.clone(),
                report.kind.as_str().to_string(),
                canister.status.clone(),
                entry.labels.join("/"),
                entry.principal.clone().unwrap_or_else(|| "-".to_string()),
                metric_value_label(&entry.value),
            ]);
        }
    }

    [
        format!(
            "Fleet: {} (network {}, metrics {})",
            report.fleet,
            report.network,
            report.kind.as_str()
        ),
        String::new(),
        render_table(
            &[
                "ROLE",
                "CANISTER_ID",
                "KIND",
                "STATUS",
                "LABELS",
                "PRINCIPAL",
                "VALUE",
            ],
            &rows,
            &[
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Right,
            ],
        ),
    ]
    .join("\n")
}

fn metric_value_label(value: &MetricValue) -> String {
    match value {
        MetricValue::Count { count } => count.to_string(),
        MetricValue::CountAndU64 { count, value_u64 } => format!("{count}/{value_u64}"),
        MetricValue::U128 { value } => value.to_string(),
    }
}
