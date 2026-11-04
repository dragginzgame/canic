//! Admit upstream-owned schemas against each final managed Blob composition.
//! Export parity does not prove target-specific payload layouts or release hash custody.

use canic_contracts::protocol::wire_declaration::{
    CompiledMethod, MethodKind, MethodLimits, SECTION,
};
use ic_blob_storage_contracts::{
    endpoints::{MethodCodec, MethodMode, SERVICE_METHODS},
    wire,
};
use ic_host_artifacts::wasm::{InspectionLimits, WasmFacts, inspect};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn qualify_retained(path: &std::path::Path) -> canic_host::canister_build::WireBindings {
    let wasm = std::fs::read(path).unwrap();
    let facts = inspect(
        &wasm,
        InspectionLimits {
            module_bytes: wasm.len(),
            sections: 128,
            exports: 512,
            custom_sections: 128,
        },
    )
    .unwrap();
    let schema = std::fs::read(path.with_extension("wire.json")).unwrap();
    let bindings =
        canic_host::canister_build::generate_wire_bindings(&facts, &schema, "./wire-adapter.js")
            .unwrap();
    assert_eq!(
        bindings.browser_modules["wire-adapter.ts"],
        wire::schema::runtime_adapter("./canic-adapter.js")
    );
    assert_eq!(
        bindings.browser_modules["wire-runtime.ts"],
        ic_wire::typescript::RUNTIME
    );
    assert_eq!(
        bindings.browser_modules["canic-adapter.ts"],
        canic_contracts::ids::wire_schema::TYPESCRIPT_ADAPTER
    );
    assert_eq!(
        std::fs::read_to_string(path.with_extension("wire.ts")).unwrap(),
        bindings.typescript
    );
    bindings
}

pub(super) fn qualify(wasm: &[u8]) {
    let facts = inspect(
        wasm,
        InspectionLimits {
            module_bytes: wasm.len(),
            sections: 128,
            exports: 512,
            custom_sections: 128,
        },
    )
    .expect("bounded final Blob Wasm inspection");
    let compiled = compiled_methods(&facts);
    let methods = wire::schema::methods().expect("published Blob native schemas");
    assert!(!methods.is_empty());
    let indexed: BTreeMap<_, _> = methods
        .iter()
        .map(|method| (method["name"].as_str().unwrap(), method))
        .collect();
    assert_eq!(indexed.len(), methods.len());
    let identities: BTreeSet<_> = methods
        .iter()
        .map(|method| method["id"].as_str().unwrap())
        .collect();
    assert_eq!(identities.len(), methods.len());

    let expected: BTreeSet<_> = SERVICE_METHODS
        .iter()
        .filter(|method| method.codec == MethodCodec::IcWire)
        .map(|method| method.name)
        .collect();
    assert_eq!(indexed.keys().copied().collect::<BTreeSet<_>>(), expected);
    for spec in SERVICE_METHODS {
        let mode = match spec.mode {
            MethodMode::Query => "query",
            MethodMode::Update => "update",
        };
        let export = format!("canister_{mode} {}", spec.name);
        assert!(
            facts.exports.contains_key(export.as_str()),
            "missing {export}"
        );
        if spec.codec == MethodCodec::Candid {
            assert!(!indexed.contains_key(spec.name));
            assert!(!compiled.contains_key(spec.name));
            continue;
        }
        let method = indexed[spec.name];
        let declaration = compiled[spec.name];
        assert_eq!(declaration.id, wire::identity(spec.name).unwrap());
        assert_eq!(
            declaration.kind,
            match spec.mode {
                MethodMode::Query => MethodKind::Query,
                MethodMode::Update => MethodKind::Update,
            }
        );
        assert_eq!(method["mode"], mode);
        assert_eq!(method["id"], wire::identity(spec.name).unwrap().to_string());
        for (direction, limits, actual) in [
            (
                "request",
                wire::request_limits(spec.name).unwrap(),
                declaration.request,
            ),
            (
                "response",
                wire::response_limits(spec.name).unwrap(),
                declaration.response,
            ),
        ] {
            assert_eq!(
                actual,
                MethodLimits {
                    bytes: limits.bytes as u64,
                    allocation: limits.allocation as u64,
                    nodes: limits.nodes as u64,
                    depth: limits.depth as u64,
                }
            );
            for (suffix, expected) in [
                ("Bytes", limits.bytes),
                ("Allocation", limits.allocation),
                ("Nodes", limits.nodes),
                ("Depth", limits.depth),
            ] {
                assert_eq!(method[format!("{direction}{suffix}")], expected);
            }
        }
    }
    // The supplying owner composes its scalar adapter; do not generate Blob against
    // Canic's generic adapter or copy its custom decimal validators downstream.
    let bindings = wire::schema::typescript("./blob-runtime.js").unwrap();
    let adapter = wire::schema::runtime_adapter("./wire-runtime.js");
    assert!(!bindings.is_empty());
    assert!(!adapter.is_empty());
}

// Other application-owned Wire declarations may coexist in an embedded artifact.
fn compiled_methods<'a>(facts: &WasmFacts<'a>) -> BTreeMap<&'a str, CompiledMethod<'a>> {
    let mut methods = BTreeMap::new();
    let mut identities = BTreeSet::new();
    for section in facts
        .custom_sections
        .iter()
        .filter(|section| section.name == SECTION)
    {
        let mut bytes = section.data;
        while !bytes.is_empty() {
            let method = CompiledMethod::decode(&mut bytes).expect("compiled Wire record");
            assert!(identities.insert(method.id));
            assert!(methods.insert(method.name, method).is_none());
        }
    }
    methods
}
