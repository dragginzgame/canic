use canic_host::{
    candid_endpoints::{EndpointEntry, EndpointMode, EndpointType, render_candid_method_name},
    table::{ColumnAlign, render_table},
};

pub(super) fn render_plain_endpoints(endpoints: &[EndpointEntry]) -> String {
    const HEADERS: [&str; 3] = ["FUNCTION", "MODE", "SIGNATURE"];
    const ALIGNMENTS: [ColumnAlign; 3] = [ColumnAlign::Left; 3];
    let rows = endpoints
        .iter()
        .map(|endpoint| {
            [
                render_endpoint_method_name(endpoint),
                render_endpoint_mode_label(endpoint),
                render_endpoint_signature(endpoint),
            ]
        })
        .collect::<Vec<_>>();

    render_table(&HEADERS, &rows, &ALIGNMENTS)
}

fn render_endpoint_type_list(types: &[EndpointType]) -> String {
    format!(
        "({})",
        types
            .iter()
            .map(endpoint_type_candid)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn render_endpoint_method_name(endpoint: &EndpointEntry) -> String {
    render_candid_method_name(&endpoint.name)
}

fn render_endpoint_mode_label(endpoint: &EndpointEntry) -> String {
    if endpoint.modes.is_empty() {
        "update".to_string()
    } else {
        endpoint
            .modes
            .iter()
            .map(endpoint_mode_candid_label)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn render_endpoint_signature(endpoint: &EndpointEntry) -> String {
    format!(
        "{} -> {}",
        render_endpoint_type_list(&endpoint.arguments),
        render_endpoint_type_list(&endpoint.returns)
    )
}

const fn endpoint_mode_candid_label(mode: &EndpointMode) -> &'static str {
    match mode {
        EndpointMode::Query => "query",
        EndpointMode::CompositeQuery => "composite_query",
        EndpointMode::Oneway => "oneway",
    }
}

fn endpoint_type_candid(endpoint_type: &EndpointType) -> &str {
    match endpoint_type {
        EndpointType::Primitive { candid, .. }
        | EndpointType::Named { candid, .. }
        | EndpointType::Optional { candid, .. }
        | EndpointType::Vector { candid, .. }
        | EndpointType::Record { candid, .. }
        | EndpointType::Variant { candid, .. }
        | EndpointType::Function { candid, .. }
        | EndpointType::Service { candid, .. }
        | EndpointType::Class { candid, .. } => candid,
    }
}
