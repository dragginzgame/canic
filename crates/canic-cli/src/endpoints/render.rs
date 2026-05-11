use crate::endpoints::model::{EndpointEntry, EndpointType};
use canic_host::table::{ColumnAlign, render_table};

pub(super) fn render_plain_endpoints(endpoints: &[EndpointEntry]) -> String {
    const HEADERS: [&str; 3] = ["FUNCTION", "MODE", "SIGNATURE"];
    const ALIGNMENTS: [ColumnAlign; 3] = [ColumnAlign::Left; 3];
    let rows = endpoints
        .iter()
        .map(|endpoint| {
            [
                endpoint.rendered_method_name(),
                endpoint.mode_label(),
                endpoint.signature(),
            ]
        })
        .collect::<Vec<_>>();

    render_table(&HEADERS, &rows, &ALIGNMENTS)
}

pub(super) fn render_endpoint_type_list(types: &[EndpointType]) -> String {
    format!(
        "({})",
        types
            .iter()
            .map(EndpointType::candid)
            .collect::<Vec<_>>()
            .join(", ")
    )
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
