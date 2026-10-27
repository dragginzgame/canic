//! Native producer admission against the actual final dedicated Blob artifact.

#[test]
#[ignore = "requires CANIC_BLOB_SCHEMA_WASM naming the managed final Wasm"]
fn mounted_supplier_schema_matches_final_wasm() {
    use canic::endpoint::schema::{self, CompiledMethod};
    use ic_host_artifacts::wasm::{InspectionLimits, inspect};
    let path = std::env::var_os("CANIC_BLOB_SCHEMA_WASM").expect("final managed Blob Wasm");
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
    let snapshot = schema::snapshot(&compiled).unwrap();
    assert!(!compiled.is_empty());
    assert_eq!(
        snapshot["methods"].as_array().unwrap().len(),
        compiled.len()
    );
    assert_eq!(snapshot, schema::snapshot(&compiled).unwrap());
    for supplier in
        canic_blob_service::__private::ic_blob_storage_contracts::wire::schema::methods().unwrap()
    {
        let name = supplier["name"].as_str().unwrap();
        for direction in ["request", "response"] {
            assert_eq!(
                snapshot["types"][format!("{name}_{direction}")],
                supplier[direction]
            );
        }
    }

    compiled[0].request.bytes += 1;
    assert!(schema::snapshot(&compiled).is_err());
}
