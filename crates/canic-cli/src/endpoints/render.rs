use super::model::EndpointEntry;
use ic_host_text::table::{self, Alignment, Options};

pub(super) fn render_plain_endpoints(endpoints: &[EndpointEntry]) -> String {
    const HEADERS: [&str; 6] = ["FUNCTION", "ID", "MODE", "REQUEST", "RESPONSE", "BUDGETS"];
    let rows = endpoints
        .iter()
        .map(|method| {
            [
                method.name.clone(),
                method.id.clone(),
                method.mode.clone(),
                method.request.to_string(),
                method.response.to_string(),
                format!(
                    "request={}/{}/{}/{} response={}/{}/{}/{}",
                    method.request_bytes,
                    method.request_allocation,
                    method.request_nodes,
                    method.request_depth,
                    method.response_bytes,
                    method.response_allocation,
                    method.response_nodes,
                    method.response_depth
                ),
            ]
        })
        .collect::<Vec<_>>();
    let mut result = table::render(
        &HEADERS,
        &rows,
        &[Alignment::Left; 6],
        Options {
            column_gap: 3,
            header_separator: true,
        },
    );
    result.push_str(
        "\nBudgets: bytes/allocation/nodes/depth; type references are retained in JSON output.",
    );
    result
}
