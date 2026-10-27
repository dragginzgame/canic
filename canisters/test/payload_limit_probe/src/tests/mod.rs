//! Compare native endpoint metadata with the same managed probe compiled to Wasm.

use canic::endpoint::schema::{self, CompiledMethod};
use ic_host_artifacts::wasm::{InspectionLimits, inspect};

#[test]
#[ignore = "requires CANIC_WIRE_PROBE_WASM pointing to the managed probe artifact"]
fn native_schema_matches_managed_wasm_declarations() {
    let path = std::env::var_os("CANIC_WIRE_PROBE_WASM").expect("managed probe Wasm path");
    let wasm = std::fs::read(path).unwrap();
    let facts = inspect(
        &wasm,
        InspectionLimits {
            module_bytes: wasm.len(),
            sections: 128,
            exports: 256,
            custom_sections: 64,
        },
    )
    .unwrap();
    let mut compiled = Vec::new();
    for section in facts
        .custom_sections
        .iter()
        .filter(|section| section.name == "icp:private canic:wire")
    {
        let mut bytes = section.data;
        while !bytes.is_empty() {
            compiled.push(CompiledMethod::decode(&mut bytes).unwrap());
        }
    }
    let schema = schema::snapshot(&compiled).unwrap();
    let methods = schema["methods"].as_array().unwrap();
    assert_eq!(
        methods
            .iter()
            .map(|method| method["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "native_commit",
            "native_composite",
            "native_principal",
            "native_raw",
            "native_result"
        ]
    );
    for method in methods {
        assert_eq!(method["requestBytes"], super::WIRE_LIMITS.bytes);
        assert_eq!(method["responseBytes"], super::WIRE_LIMITS.bytes);
    }
    assert_eq!(schema["types"]["native_commit_request"]["k"], "u");
    assert_eq!(
        schema["types"]["native_principal_request"],
        ic_wire::typescript::native_schema::<ic_platform::Principal>().unwrap()
    );
    compiled[0].id += 1;
    assert_eq!(
        schema::snapshot(&compiled),
        Err(ic_wire::typescript::GenerationError::Schema)
    );
}
