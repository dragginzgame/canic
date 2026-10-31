//! Compile real endpoint declarations and read their native artifact registry.
#![cfg(feature = "wire-schema")]

use canic::endpoint::schema;
use ic_platform::Principal;
use ic_wire::{Limits, typescript};
use serde_json::json;

const IDENTITY_METHOD_ID: u64 = 9001;

const REQUEST: Limits = Limits {
    bytes: 128,
    allocation: 256,
    nodes: 16,
    depth: 8,
};
const RESPONSE: Limits = Limits {
    bytes: 256,
    allocation: 512,
    nodes: 32,
    depth: 16,
};

#[canic::query(public, name = "identity", on_access_denied = "reject",
    wire(id = crate::IDENTITY_METHOD_ID, request = REQUEST, response = RESPONSE))]
fn echo(principal: Principal) -> Principal {
    principal
}

#[canic::query(public, composite, on_access_denied = "reject",
    wire(id = 9002, request = REQUEST, response = RESPONSE))]
fn observed() -> u64 {
    7
}

#[canic::update(public, on_access_denied = "reject",
    wire(id = 9003, request = REQUEST, response = RESPONSE))]
fn write(value: Option<u64>) -> Option<u64> {
    value
}

#[test]
fn declarations_supply_exact_types_kinds_names_ids_and_budgets() {
    let limits = |limits: Limits| schema::MethodLimits {
        bytes: limits.bytes as u64,
        allocation: limits.allocation as u64,
        nodes: limits.nodes as u64,
        depth: limits.depth as u64,
    };
    let compiled = [
        ("identity", schema::MethodKind::Query, 9001),
        ("observed", schema::MethodKind::CompositeQuery, 9002),
        ("write", schema::MethodKind::Update, 9003),
    ]
    .map(|(name, kind, id)| schema::CompiledMethod {
        name,
        kind,
        id,
        request: limits(REQUEST),
        response: limits(RESPONSE),
    });
    let schema = schema::snapshot(&compiled).unwrap();
    let methods = schema["methods"].as_array().unwrap();
    assert_eq!(
        methods
            .iter()
            .map(|m| m["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["identity", "observed", "write"]
    );
    assert_eq!(methods[0]["id"], "9001");
    assert_eq!(methods[1]["kind"], "composite_query");
    assert_eq!(methods[1]["mode"], "query");
    assert_eq!(methods[2]["kind"], "update");
    for method in methods {
        assert_eq!(method["requestBytes"], REQUEST.bytes);
        assert_eq!(method["requestAllocation"], REQUEST.allocation);
        assert_eq!(method["requestNodes"], REQUEST.nodes);
        assert_eq!(method["requestDepth"], REQUEST.depth);
        assert_eq!(method["responseBytes"], RESPONSE.bytes);
        assert_eq!(method["responseAllocation"], RESPONSE.allocation);
        assert_eq!(method["responseNodes"], RESPONSE.nodes);
        assert_eq!(method["responseDepth"], RESPONSE.depth);
    }
    assert_eq!(
        schema["types"]["identity_request"],
        typescript::native_schema::<Principal>().unwrap()
    );
    assert_eq!(schema["types"]["observed_request"], json!({"k":"unit"}));
    assert_eq!(
        schema["types"]["write_response"],
        typescript::native_schema::<Option<u64>>().unwrap()
    );
    let types = serde_json::from_value(schema["types"].clone()).unwrap();
    let generated = typescript::generate_registry(
        &types,
        methods,
        "./adapter.js",
        &std::collections::BTreeMap::new(),
        |report| eprint!("{report}"),
    )
    .unwrap();
    assert!(generated.contains("export const identity"));
    assert!(generated.contains("export const observed"));
}
