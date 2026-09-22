use std::fmt::Write;

use canic_host::{
    candid_endpoints::{
        EndpointEntry, EndpointMode, EndpointType, IngressPayloadBasis, render_candid_method_name,
    },
    table::{ColumnAlign, render_table},
};

pub(super) fn render_plain_endpoints(endpoints: &[EndpointEntry]) -> String {
    const HEADERS: [&str; 3] = ["FUNCTION", "MODE", "SIGNATURE"];
    const ALIGNMENTS: [ColumnAlign; 3] = [ColumnAlign::Left; 3];
    let rows = endpoints
        .iter()
        .map(|endpoint| {
            [
                render_candid_method_name(&endpoint.name),
                render_endpoint_mode_label(endpoint),
                render_endpoint_signature(endpoint),
            ]
        })
        .collect::<Vec<_>>();

    let mut table = render_table(&HEADERS, &rows, &ALIGNMENTS);
    for endpoint in endpoints.iter().filter(|endpoint| {
        !endpoint
            .modes
            .iter()
            .any(|mode| matches!(mode, EndpointMode::Query | EndpointMode::CompositeQuery))
    }) {
        let limits = endpoint.payload_limits.as_ref();
        let ingress = limits
            .and_then(|limits| limits.ingress_max_bytes)
            .map_or_else(
                || match limits.map(|limits| limits.ingress_basis) {
                    Some(IngressPayloadBasis::VariantDependent) => "variant-dependent".into(),
                    _ => "unknown".into(),
                },
                |bytes| format!("{bytes} bytes"),
            );
        let guard = limits
            .and_then(|limits| limits.update_guard_max_bytes)
            .map_or_else(
                || {
                    if limits.is_some() {
                        "not declared"
                    } else {
                        "unknown"
                    }
                    .into()
                },
                |bytes| format!("{bytes} bytes"),
            );
        write!(
            table,
            "\n{}: ingress={ingress}; update argument guard={guard}",
            render_candid_method_name(&endpoint.name)
        )
        .expect("format endpoint limits");
    }
    table
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
