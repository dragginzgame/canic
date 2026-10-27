//! Final-artifact identity checks, without inventing payload schemas downstream.

use super::*;
use canic_contracts::protocol::wire_declaration::{HEADER_BYTES, MethodLimits, SECTION};
use ic_host_artifacts::wasm::{CustomSection, InspectionLimits, inspect};

const LIMITS: MethodLimits = MethodLimits {
    bytes: 128,
    allocation: 256,
    nodes: 16,
    depth: 8,
};
const METHOD: CompiledMethod<'_> = CompiledMethod {
    name: "echo",
    kind: MethodKind::Query,
    id: 9001,
    request: LIMITS,
    response: LIMITS,
};
const BYTES: [u8; HEADER_BYTES + 4] = METHOD.encode();
// One exported query, built from standard Wasm sections; Host's parser owns admission.
const WASM: &[u8] = b"\0asm\x01\0\0\0\x01\x04\x01\x60\0\0\x03\x02\x01\0\x07\x17\x01\x13canister_query echo\0\0\x0a\x04\x01\x02\0\x0b";

fn facts(bytes: &[u8]) -> WasmFacts<'_> {
    let mut facts = inspect(
        WASM,
        InspectionLimits {
            module_bytes: 4096,
            sections: 10,
            exports: 10,
            custom_sections: 10,
        },
    )
    .unwrap();
    facts.custom_sections.push(CustomSection {
        name: SECTION,
        data: bytes,
    });
    facts
}

#[test]
fn compiled_records_reject_duplicate_identity_wrong_exports_and_truncation() {
    assert_eq!(
        declarations(&facts(&[BYTES, BYTES].concat())),
        Err(WireArtifactError::Duplicate)
    );
    let wrong = CompiledMethod {
        kind: MethodKind::Update,
        ..METHOD
    }
    .encode::<{ HEADER_BYTES + 4 }>();
    assert_eq!(declarations(&facts(&wrong)), Err(WireArtifactError::Export));
    assert_eq!(
        declarations(&facts(&BYTES[..BYTES.len() - 1])),
        Err(WireArtifactError::Declaration(DeclarationError::Framing))
    );
}

#[test]
fn transform_cannot_change_identity_or_budget_while_retaining_exports() {
    let before = facts(&BYTES);
    assert_eq!(validate_transform(&before, &facts(&BYTES)), Ok(()));
    for changed in [
        CompiledMethod { id: 9002, ..METHOD },
        CompiledMethod {
            response: MethodLimits {
                allocation: 257,
                ..LIMITS
            },
            ..METHOD
        },
    ] {
        let bytes = changed.encode::<{ HEADER_BYTES + 4 }>();
        assert_eq!(
            validate_transform(&before, &facts(&bytes)),
            Err(WireArtifactError::Contract)
        );
    }
    let mut removed = facts(&BYTES);
    removed.custom_sections.clear();
    assert_eq!(
        validate_transform(&before, &removed),
        Err(WireArtifactError::Contract)
    );
}

fn schema() -> serde_json::Value {
    serde_json::json!({"schema_version":1, "browser_modules": {
        "canic-adapter.ts":"export {};", "identities.ts":"export {};",
        "wire-adapter.ts":"export {};", "wire-runtime.ts":"export {};"
    }, "types":{
        "request":{"k":"unit"}, "response":{"k":"u", "bits":64}
    }, "methods":[{
        "name":"echo", "id":"9001", "kind":"query", "mode":"query",
        "request":{"k":"ref", "name":"request"},
        "response":{"k":"ref", "name":"response"},
        "requestBytes":128, "requestAllocation":256, "requestNodes":16, "requestDepth":8,
        "responseBytes":128, "responseAllocation":256, "responseNodes":16, "responseDepth":8
    }]})
}

fn generate(schema: &serde_json::Value) -> Result<WireBindings, WireArtifactError> {
    generate_wire_bindings(
        &facts(&BYTES),
        &serde_json::to_vec(schema).unwrap(),
        "./adapter.js",
    )
}

#[test]
fn native_schema_admission_preserves_wire_generation_and_refuses_contract_drift() {
    let original = schema();
    let expected = ic_wire::typescript::generate_registry(
        &serde_json::from_value(original["types"].clone()).unwrap(),
        original["methods"].as_array().unwrap(),
        "./adapter.js",
        &std::collections::BTreeMap::new(),
        |report| eprint!("{report}"),
    )
    .unwrap();
    assert_eq!(generate(&original).unwrap().typescript, expected);
    for (field, value) in [
        ("name", serde_json::json!("other")),
        ("id", serde_json::json!("9002")),
        ("kind", serde_json::json!("composite_query")),
        ("mode", serde_json::json!("update")),
        ("requestBytes", serde_json::json!(129)),
        ("requestAllocation", serde_json::json!(257)),
        ("requestNodes", serde_json::json!(17)),
        ("requestDepth", serde_json::json!(9)),
        ("responseBytes", serde_json::json!(129)),
        ("responseAllocation", serde_json::json!(257)),
        ("responseNodes", serde_json::json!(17)),
        ("responseDepth", serde_json::json!(9)),
    ] {
        let mut changed = original.clone();
        changed["methods"][0][field] = value;
        assert_eq!(
            generate(&changed),
            Err(WireArtifactError::Contract),
            "{field}"
        );
    }
}

#[test]
fn schema_identity_binds_exact_bytes_independently_of_browser_module_selection() {
    let compact = serde_json::to_vec(&schema()).unwrap();
    let pretty = serde_json::to_vec_pretty(&schema()).unwrap();
    let admit =
        |bytes: &[u8], module: &str| generate_wire_bindings(&facts(&BYTES), bytes, module).unwrap();
    let first = admit(&compact, "./adapter.js");
    let formatted = admit(&pretty, "./adapter.js");
    let relocated = admit(&compact, "./relocated.js");
    assert_eq!(
        first.schema_sha256,
        <[u8; 32]>::from(Sha256::digest(&compact))
    );
    assert_ne!(first.schema_sha256, formatted.schema_sha256);
    assert_eq!(first.typescript, formatted.typescript);
    assert_eq!(first.schema_sha256, relocated.schema_sha256);
    assert_ne!(first.typescript, relocated.typescript);
}

#[test]
fn native_schema_admission_rejects_invalid_documents_and_missing_references() {
    let mut invalid = schema();
    invalid["schema_version"] = 2.into();
    assert_eq!(generate(&invalid), Err(WireArtifactError::Schema));
    invalid = schema();
    invalid["types"] = serde_json::json!({});
    assert_eq!(generate(&invalid), Err(WireArtifactError::Schema));
    invalid = schema();
    invalid["methods"] = serde_json::json!([]);
    assert_eq!(generate(&invalid), Err(WireArtifactError::Contract));
    assert_eq!(
        generate_wire_bindings(&facts(&BYTES), b"{", "./adapter.js"),
        Err(WireArtifactError::Schema)
    );
    assert_eq!(
        generate_wire_bindings(
            &facts(&BYTES),
            &vec![b' '; crate::MAX_DOCUMENT_READ_BYTES + 1],
            "./adapter.js"
        ),
        Err(WireArtifactError::Schema)
    );
}

#[test]
fn browser_sources_are_release_bound_without_current_host_substitution() {
    let original = schema();
    let first = generate(&original).unwrap();
    let mut changed = original.clone();
    changed["browser_modules"]["wire-runtime.ts"] = serde_json::json!("export const retained = 7;");
    let second = generate(&changed).unwrap();
    assert_ne!(first.schema_sha256, second.schema_sha256);
    assert_eq!(
        second.browser_modules["wire-runtime.ts"],
        "export const retained = 7;"
    );
    assert_eq!(first.typescript, second.typescript);
    for modules in [
        serde_json::Value::Null,
        serde_json::json!({}),
        {
            let mut value = original["browser_modules"].clone();
            value["wire-adapter.ts"] = serde_json::json!(" ");
            value
        },
        {
            let mut value = original["browser_modules"].clone();
            value["../escape.ts"] = serde_json::json!("export {};");
            value
        },
    ] {
        let mut invalid = original.clone();
        invalid["browser_modules"] = modules;
        assert_eq!(generate(&invalid), Err(WireArtifactError::Schema));
    }
}

#[test]
fn retained_method_selection_requires_exact_identity_mode_and_allowances() {
    let limits = ic_wire::Limits {
        bytes: 128,
        allocation: 256,
        nodes: 16,
        depth: 8,
    };
    let admit = |schema: &Value| {
        require_wire_method(
            &serde_json::to_vec(schema).unwrap(),
            "echo",
            9001,
            MethodKind::Query,
            limits,
            limits,
        )
    };
    assert_eq!(admit(&schema()), Ok(()));
    for (field, value) in [
        ("name", serde_json::json!("other")),
        ("id", serde_json::json!("9002")),
        ("kind", serde_json::json!("update")),
        ("mode", serde_json::json!("update")),
        ("requestBytes", serde_json::json!(129)),
        ("responseNodes", serde_json::json!(17)),
    ] {
        let mut changed = schema();
        changed["methods"][0][field] = value;
        assert_eq!(admit(&changed), Err(WireArtifactError::Contract));
    }
    let mut duplicate = schema();
    let method = duplicate["methods"][0].clone();
    duplicate["methods"].as_array_mut().unwrap().push(method);
    assert_eq!(admit(&duplicate), Err(WireArtifactError::Duplicate));
    let mut missing = schema();
    missing["methods"] = serde_json::json!([]);
    assert_eq!(admit(&missing), Err(WireArtifactError::Contract));
}
