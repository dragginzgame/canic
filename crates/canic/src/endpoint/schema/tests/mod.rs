//! Exact identities and deterministic metadata at the artifact boundary.

use super::*;

const LIMITS: Limits = Limits {
    bytes: 128,
    allocation: 256,
    nodes: 16,
    depth: 8,
};

fn method(name: &'static str, id: u64) -> Method {
    Method::describe::<(), u64>(name, MethodKind::Query, id, LIMITS, LIMITS).unwrap()
}

fn matching(methods: Vec<Method>) -> Result<Value, GenerationError> {
    let compiled: Vec<_> = methods.iter().map(Method::declaration).collect();
    assemble(methods, &compiled)
}

#[test]
fn registry_is_sorted_and_preserves_limits_and_native_types() {
    let left = matching(vec![method("zeta", 1), method("alpha", 2)]).unwrap();
    let right = matching(vec![method("alpha", 2), method("zeta", 1)]).unwrap();
    assert_eq!(
        serde_json::to_vec(&left).unwrap(),
        serde_json::to_vec(&right).unwrap()
    );
    assert_eq!(left["schema_version"], 1);
    assert_eq!(left["methods"][0]["name"], "alpha");
    assert_eq!(left["methods"][0]["id"], "2");
    assert_eq!(left["methods"][0]["requestAllocation"], 256);
    assert_eq!(left["methods"][0]["responseAllocation"], 256);
    assert_eq!(left["types"]["alpha_request"], json!({"k":"unit"}));
    assert_eq!(left["types"]["alpha_response"], json!({"k":"u", "bits":64}));
}

#[test]
fn registry_refuses_ambiguous_or_unprojectable_declarations() {
    for methods in [
        vec![],
        vec![method("same", 1), method("same", 2)],
        vec![method("first", 1), method("second", 1)],
        vec![method("invalid-name", 1)],
    ] {
        assert_eq!(matching(methods), Err(GenerationError::Schema));
    }
    let mut invalid = method("bounded", 1);
    invalid.declaration.response.bytes = 0;
    assert_eq!(matching(vec![invalid]), Err(GenerationError::Limits));
    let mut invalid = method("missing", 1);
    invalid.response = json!({"k":"ref", "name":"absent"});
    assert_eq!(matching(vec![invalid]), Err(GenerationError::Schema));
}

#[test]
fn registry_requires_exact_compiled_identity_and_both_budgets() {
    let original = method("echo", 7).declaration();
    let mut alternatives = vec![
        CompiledMethod {
            name: "other",
            ..original
        },
        CompiledMethod { id: 8, ..original },
        CompiledMethod {
            kind: MethodKind::CompositeQuery,
            ..original
        },
        CompiledMethod {
            kind: MethodKind::Update,
            ..original
        },
    ];
    for response in [false, true] {
        for field in 0..4 {
            let mut changed = original;
            let limits = if response {
                &mut changed.response
            } else {
                &mut changed.request
            };
            match field {
                0 => limits.bytes += 1,
                1 => limits.allocation += 1,
                2 => limits.nodes += 1,
                _ => limits.depth += 1,
            }
            alternatives.push(changed);
        }
    }
    for changed in alternatives {
        assert_eq!(
            assemble(vec![method("echo", 7)], &[changed]),
            Err(GenerationError::Schema)
        );
    }
    for inventory in [
        vec![],
        vec![original, original],
        vec![
            original,
            CompiledMethod {
                name: "extra",
                id: 8,
                ..original
            },
        ],
    ] {
        assert_eq!(
            assemble(vec![method("echo", 7)], &inventory),
            Err(GenerationError::Schema)
        );
    }
}

#[test]
fn supplier_projection_joins_typed_methods_without_bypassing_admission() {
    let supplier = || {
        Method::project(
            method("supplier", 19).declaration(),
            json!({"k":"text", "check":"supplierScalar"}),
            json!({"k":"unit"}),
        )
    };
    let schema = matching(vec![supplier(), method("typed", 20)]).unwrap();
    assert_eq!(
        schema["types"]["supplier_request"]["check"],
        "supplierScalar"
    );
    assert_eq!(schema["methods"][1]["name"], "typed");
    assert_eq!(
        matching(vec![supplier(), method("supplier", 20)]),
        Err(GenerationError::Schema)
    );
    assert_eq!(
        matching(vec![supplier(), method("typed", 19)]),
        Err(GenerationError::Schema)
    );
    let mut invalid = supplier();
    invalid.request = json!({"k":"ref", "name":"missing"});
    assert_eq!(matching(vec![invalid]), Err(GenerationError::Schema));
    assert_eq!(
        assemble(vec![supplier()], &[]),
        Err(GenerationError::Schema)
    );
}

#[test]
fn producer_retains_exact_runtime_and_explicit_supplier_adapter() {
    let provider = Provider {
        methods: || Ok(vec![method("echo", 7)]),
        browser_adapter: None,
    };
    let compiled = [method("echo", 7).declaration()];
    let plain = project(vec![provider], Some(&compiled)).unwrap();
    assert_eq!(
        plain["browser_modules"]["wire-runtime.ts"],
        typescript::RUNTIME
    );
    assert_eq!(
        plain["browser_modules"]["identities.ts"],
        canic_contracts::ids::wire_schema::TYPESCRIPT_VALIDATOR
    );
    let adapted = Provider {
        browser_adapter: Some(|base| format!("export {{ createCodec, WireError }} from {base:?};")),
        ..provider
    };
    let selected = project(vec![adapted], Some(&compiled)).unwrap();
    assert_eq!(
        selected["browser_modules"]["wire-adapter.ts"],
        "export { createCodec, WireError } from \"./canic-adapter.js\";"
    );
    assert_ne!(plain, selected);
    assert_eq!(
        project(vec![adapted, adapted], Some(&compiled)),
        Err(GenerationError::Schema)
    );
}

#[test]
fn native_contract_matches_final_admission_and_rejects_changed_declarations() {
    let provider = Provider {
        methods: || Ok(vec![method("echo", 7)]),
        browser_adapter: None,
    };
    let native = project(vec![provider], None).unwrap();
    let matching = [method("echo", 7).declaration()];
    assert_eq!(native, project(vec![provider], Some(&matching)).unwrap());
    let changed = [method("echo", 8).declaration()];
    assert_eq!(
        project(vec![provider], Some(&changed)),
        Err(GenerationError::Schema)
    );
    assert_eq!(
        project(vec![provider], Some(&[])),
        Err(GenerationError::Schema)
    );
    assert_eq!(
        project(vec![provider, provider], None),
        Err(GenerationError::Schema)
    );
}
