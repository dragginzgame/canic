//! Module: canic_cli::info_subnets::render
//!
//! Responsibility: render validated Fleet Subnet inventory as operator text.
//! Does not own: evidence validation, JSON schema, or network queries.
//! Boundary: consumes only a complete report and never represents partial query state.

use crate::info_subnets::model::FleetSubnetInventoryReportV1;

use canic_host::table::{ColumnAlign, render_table};

pub(super) fn text_report(report: &FleetSubnetInventoryReportV1) -> String {
    let rows = report
        .subnets
        .iter()
        .map(|row| {
            [
                row.subnet.clone(),
                row.root.clone().unwrap_or_else(|| "-".to_string()),
                row.status
                    .as_deref()
                    .map_or_else(|| "-".to_string(), str::to_ascii_uppercase),
                row.total_canisters.to_string(),
            ]
        })
        .collect::<Vec<_>>();
    let table = render_table(
        &["SUBNET", "ROOT", "STATUS", "CANISTERS"],
        &rows,
        &[
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Right,
        ],
    );
    format!(
        "{table}\n\nFleet total: {} Canisters",
        report.total_canisters
    )
}
