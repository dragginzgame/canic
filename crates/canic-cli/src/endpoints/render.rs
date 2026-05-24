use crate::endpoints::model::{EndpointEntry, EndpointMode, EndpointType};
use canic_host::table::{ColumnAlign, render_table};

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

pub(super) fn render_candid_method_name(name: &str) -> String {
    if is_candid_identifier(name) && !is_candid_reserved_word(name) {
        name.to_string()
    } else {
        format!("{name:?}")
    }
}

fn is_candid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn is_candid_reserved_word(name: &str) -> bool {
    matches!(
        name,
        "blob"
            | "bool"
            | "composite_query"
            | "empty"
            | "false"
            | "float32"
            | "float64"
            | "func"
            | "import"
            | "int"
            | "int8"
            | "int16"
            | "int32"
            | "int64"
            | "nat"
            | "nat8"
            | "nat16"
            | "nat32"
            | "nat64"
            | "null"
            | "oneway"
            | "opt"
            | "principal"
            | "query"
            | "record"
            | "reserved"
            | "service"
            | "text"
            | "true"
            | "type"
            | "variant"
            | "vec"
    )
}
