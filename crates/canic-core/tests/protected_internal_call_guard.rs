// Category C - Source guard test (no embedded config).

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use canic_core::protocol::{
    CANIC_WASM_STORE_PROTECTED_UPDATE_METHODS, CANIC_WASM_STORE_STRUCTURAL_QUERY_METHODS,
};

const FIRST_PARTY_SRC_ROOTS: &[&str] = &[
    "crates/canic/src",
    "crates/canic-core/src",
    "crates/canic-control-plane/src",
    "crates/canic-wasm-store/src",
    "canisters",
    "fleets",
];

const RAW_CALL_PATTERNS: &[&str] = &["Call::", "CallOps::", "ic_cdk::call::Call"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InternalEndpointClass {
    ProtectedInternalRpc,
    StructuralBootstrap,
    StructuralQueryException,
    ExistingCapabilityRpc,
    Discovery,
    OperatorRawPath,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct InternalEndpointClassification {
    method: &'static str,
    class: InternalEndpointClass,
}

const INTERNAL_ENDPOINT_CLASSIFICATIONS: &[InternalEndpointClassification] = &[
    InternalEndpointClassification {
        method: "canic_app",
        class: InternalEndpointClass::OperatorRawPath,
    },
    InternalEndpointClassification {
        method: "canic_request_delegation",
        class: InternalEndpointClass::StructuralBootstrap,
    },
    InternalEndpointClassification {
        method: "canic_request_role_attestation",
        class: InternalEndpointClass::StructuralBootstrap,
    },
    InternalEndpointClassification {
        method: "canic_request_internal_invocation_proof",
        class: InternalEndpointClass::StructuralBootstrap,
    },
    InternalEndpointClassification {
        method: "canic_attestation_key_set",
        class: InternalEndpointClass::StructuralBootstrap,
    },
    InternalEndpointClassification {
        method: "canic_wasm_store_catalog",
        class: InternalEndpointClass::StructuralQueryException,
    },
    InternalEndpointClassification {
        method: "canic_wasm_store_prepare",
        class: InternalEndpointClass::ProtectedInternalRpc,
    },
    InternalEndpointClassification {
        method: "canic_wasm_store_stage_manifest",
        class: InternalEndpointClass::ProtectedInternalRpc,
    },
    InternalEndpointClassification {
        method: "canic_wasm_store_publish_chunk",
        class: InternalEndpointClass::ProtectedInternalRpc,
    },
    InternalEndpointClassification {
        method: "canic_wasm_store_info",
        class: InternalEndpointClass::ProtectedInternalRpc,
    },
    InternalEndpointClassification {
        method: "canic_wasm_store_status",
        class: InternalEndpointClass::StructuralQueryException,
    },
    InternalEndpointClassification {
        method: "canic_wasm_store_prepare_gc",
        class: InternalEndpointClass::ProtectedInternalRpc,
    },
    InternalEndpointClassification {
        method: "canic_wasm_store_begin_gc",
        class: InternalEndpointClass::ProtectedInternalRpc,
    },
    InternalEndpointClassification {
        method: "canic_wasm_store_complete_gc",
        class: InternalEndpointClass::ProtectedInternalRpc,
    },
    InternalEndpointClassification {
        method: "canic_wasm_store_chunk",
        class: InternalEndpointClass::ProtectedInternalRpc,
    },
    InternalEndpointClassification {
        method: "canic_ready",
        class: InternalEndpointClass::Discovery,
    },
    InternalEndpointClassification {
        method: "canic_bootstrap_status",
        class: InternalEndpointClass::Discovery,
    },
    InternalEndpointClassification {
        method: "icrc10_supported_standards",
        class: InternalEndpointClass::Discovery,
    },
    InternalEndpointClassification {
        method: "icrc21_canister_call_consent_message",
        class: InternalEndpointClass::Discovery,
    },
    InternalEndpointClassification {
        method: "canic_metadata",
        class: InternalEndpointClass::Discovery,
    },
    InternalEndpointClassification {
        method: "canic_response_capability_v1",
        class: InternalEndpointClass::ExistingCapabilityRpc,
    },
    InternalEndpointClassification {
        method: "canic_sync_state",
        class: InternalEndpointClass::StructuralBootstrap,
    },
    InternalEndpointClassification {
        method: "canic_sync_topology",
        class: InternalEndpointClass::StructuralBootstrap,
    },
];

#[test]
fn first_party_code_does_not_raw_call_protected_internal_methods() {
    let workspace = workspace_root();
    let mut violations = Vec::new();

    for root in FIRST_PARTY_SRC_ROOTS {
        scan_dir(&workspace.join(root), &mut violations);
    }

    assert!(
        violations.is_empty(),
        "protected Canic internal methods must be called through CanicCall: {violations:#?}"
    );
}

#[test]
fn first_party_internal_endpoints_have_explicit_0_40_classification() {
    let declared = declared_internal_endpoints();
    let classified = classification_map();
    let declared_methods = declared.keys().copied().collect::<BTreeSet<_>>();
    let classified_methods = classified.keys().copied().collect::<BTreeSet<_>>();
    let missing = declared_methods
        .difference(&classified_methods)
        .collect::<Vec<_>>();
    let stale = classified_methods
        .difference(&declared_methods)
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "first-party internal endpoints must be classified for the 0.40 protected-call model: {missing:#?}"
    );
    assert!(
        stale.is_empty(),
        "0.40 internal endpoint classifications must correspond to declared first-party internal endpoints: {stale:#?}"
    );

    for (method, declarations) in declared {
        let class = classified
            .get(method)
            .expect("declared method was checked above");
        for declaration in declarations {
            assert_class_matches_declaration(method, *class, &declaration.attribute);
        }
    }
}

#[test]
fn wasm_store_macro_declarations_match_protected_method_manifest() {
    let source = read_workspace_file("crates/canic/src/macros/endpoints/wasm_store.rs");
    let protected_methods = protected_wasm_store_methods_declared_by_macro(&source);
    let structural_query_methods = structural_wasm_store_queries_declared_by_macro(&source);

    assert_eq!(
        protected_methods,
        method_set(CANIC_WASM_STORE_PROTECTED_UPDATE_METHODS),
        "macro-declared protected wasm-store update methods must match the protocol manifest"
    );
    assert_eq!(
        structural_query_methods,
        method_set(CANIC_WASM_STORE_STRUCTURAL_QUERY_METHODS),
        "macro-declared structural wasm-store query exceptions must match the protocol manifest"
    );

    for method in CANIC_WASM_STORE_PROTECTED_UPDATE_METHODS {
        let declaration = declaration_prefix(&source, method);
        assert!(
            declaration.contains("canic_update(internal, requires(caller::has_role(\"root\"))"),
            "{method} must be declared as a protected root-role update"
        );
    }

    for method in CANIC_WASM_STORE_STRUCTURAL_QUERY_METHODS {
        let declaration = declaration_prefix(&source, method);
        assert!(
            declaration.contains("canic_query(internal, requires(caller::is_root()))"),
            "{method} must remain an explicit structural query exception"
        );
    }
}

#[test]
fn wasm_store_did_matches_protected_method_manifest() {
    let did = read_workspace_file("crates/canic-wasm-store/wasm_store.did");
    let protected_methods = protected_wasm_store_methods_in_did(&did);
    let structural_query_methods = structural_wasm_store_queries_in_did(&did);

    assert_eq!(
        protected_methods,
        method_set(CANIC_WASM_STORE_PROTECTED_UPDATE_METHODS),
        "did protected wasm-store update methods must match the protocol manifest"
    );
    assert_eq!(
        structural_query_methods,
        method_set(CANIC_WASM_STORE_STRUCTURAL_QUERY_METHODS),
        "did structural wasm-store query exceptions must match the protocol manifest"
    );

    for method in CANIC_WASM_STORE_PROTECTED_UPDATE_METHODS {
        let line = did_service_line(&did, method);
        assert!(
            line.contains("(CanicInternalCallEnvelopeV1)"),
            "{method} must expose the protected internal-call envelope ABI: {line}"
        );
        assert!(
            !line.contains(" query"),
            "{method} must be an update method in the protected ABI: {line}"
        );
    }

    for method in CANIC_WASM_STORE_STRUCTURAL_QUERY_METHODS {
        let line = did_service_line(&did, method);
        assert!(
            line.contains(" : () -> "),
            "{method} must keep the raw query ABI while protected queries are out of scope: {line}"
        );
        assert!(
            line.contains(" query"),
            "{method} must remain a structural query exception: {line}"
        );
    }
}

fn scan_dir(root: &Path, violations: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir(&path, violations);
            continue;
        }

        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }

        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };

        for (index, line) in contents.lines().enumerate() {
            if is_allowed_boundary(&path) {
                continue;
            }
            if RAW_CALL_PATTERNS
                .iter()
                .any(|pattern| line.contains(pattern))
                && CANIC_WASM_STORE_PROTECTED_UPDATE_METHODS
                    .iter()
                    .any(|method| line.contains(method))
            {
                violations.push(format!("{}:{}", path.display(), index + 1));
            }
        }
    }
}

#[derive(Debug)]
struct InternalEndpointDeclaration {
    attribute: String,
}

fn declared_internal_endpoints() -> BTreeMap<&'static str, Vec<InternalEndpointDeclaration>> {
    let mut declarations = BTreeMap::new();

    for relative in [
        "crates/canic/src/macros/endpoints/root.rs",
        "crates/canic/src/macros/endpoints/shared.rs",
        "crates/canic/src/macros/endpoints/nonroot.rs",
        "crates/canic/src/macros/endpoints/wasm_store.rs",
    ] {
        let source = read_workspace_file(relative);
        collect_internal_endpoint_declarations(&source, &mut declarations);
    }

    declarations
}

fn collect_internal_endpoint_declarations(
    source: &str,
    declarations: &mut BTreeMap<&'static str, Vec<InternalEndpointDeclaration>>,
) {
    let mut offset = 0;
    while let Some(attribute_offset) = source[offset..].find("#[$crate::canic_") {
        let attribute_start = offset + attribute_offset;
        let Some(attribute_end_offset) = source[attribute_start..].find(']') else {
            break;
        };
        let attribute_end = attribute_start + attribute_end_offset + 1;
        let attribute = &source[attribute_start..attribute_end];
        offset = attribute_end;

        if !attribute.contains("internal") {
            continue;
        }

        let Some(name) = next_function_name(&source[attribute_end..]) else {
            continue;
        };
        declarations
            .entry(leak_test_str(name))
            .or_default()
            .push(InternalEndpointDeclaration {
                attribute: attribute.to_string(),
            });
    }
}

fn next_function_name(source: &str) -> Option<&str> {
    let fn_index = source.find("fn ")?;
    let after_fn = &source[(fn_index + 3)..];
    let name_len = after_fn
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .map(char::len_utf8)
        .sum();
    Some(&after_fn[..name_len])
}

fn classification_map() -> BTreeMap<&'static str, InternalEndpointClass> {
    let mut classifications = BTreeMap::new();
    for classification in INTERNAL_ENDPOINT_CLASSIFICATIONS {
        let previous = classifications.insert(classification.method, classification.class);
        assert!(
            previous.is_none(),
            "duplicate 0.40 internal endpoint classification for {}",
            classification.method
        );
    }
    classifications
}

fn assert_class_matches_declaration(method: &str, class: InternalEndpointClass, attribute: &str) {
    match class {
        InternalEndpointClass::ProtectedInternalRpc => assert!(
            attribute.contains("canic_update(internal")
                && attribute.contains("caller::has_role")
                && CANIC_WASM_STORE_PROTECTED_UPDATE_METHODS.contains(&method),
            "{method} must be a protected update endpoint: {attribute}"
        ),
        InternalEndpointClass::StructuralBootstrap => assert!(
            attribute.contains("canic_update(internal") && !attribute.contains("caller::has_role"),
            "{method} must remain a structural bootstrap update exception: {attribute}"
        ),
        InternalEndpointClass::StructuralQueryException => assert!(
            attribute.contains("canic_query(internal")
                && CANIC_WASM_STORE_STRUCTURAL_QUERY_METHODS.contains(&method),
            "{method} must remain a structural query exception: {attribute}"
        ),
        InternalEndpointClass::ExistingCapabilityRpc => assert!(
            attribute.contains("canic_update(internal") && !attribute.contains("caller::has_role"),
            "{method} must remain on the existing capability-RPC path: {attribute}"
        ),
        InternalEndpointClass::Discovery => assert!(
            attribute.contains("canic_query(internal"),
            "{method} must remain a read-only discovery/standards endpoint: {attribute}"
        ),
        InternalEndpointClass::OperatorRawPath => assert!(
            attribute.contains("canic_update(internal")
                && attribute.contains("caller::is_controller"),
            "{method} must remain an operator/controller raw endpoint: {attribute}"
        ),
    }
}

fn leak_test_str(value: &str) -> &'static str {
    Box::leak(value.to_string().into_boxed_str())
}

fn is_allowed_boundary(path: &Path) -> bool {
    path.to_string_lossy()
        .ends_with("/crates/canic-core/src/api/ic/call.rs")
}

fn declaration_prefix<'a>(source: &'a str, method: &str) -> &'a str {
    let needle = format!("async fn {method}");
    let function_start = source
        .find(&needle)
        .unwrap_or_else(|| panic!("missing wasm-store endpoint declaration for {method}"));
    let prefix = &source[..function_start];
    let attribute_start = prefix
        .rfind("#[$crate::canic_")
        .unwrap_or_else(|| panic!("missing endpoint attribute for {method}"));
    &source[attribute_start..function_start]
}

fn did_service_line<'a>(did: &'a str, method: &str) -> &'a str {
    did.lines()
        .find(|line| line.trim_start().starts_with(&format!("{method} : ")))
        .unwrap_or_else(|| panic!("missing wasm-store did service method {method}"))
}

fn protected_wasm_store_methods_declared_by_macro(source: &str) -> BTreeSet<&'static str> {
    wasm_store_methods_declared_by_macro(source)
        .into_iter()
        .filter(|(_, attribute)| attribute.contains("caller::has_role(\"root\")"))
        .map(|(method, _)| method)
        .collect()
}

fn structural_wasm_store_queries_declared_by_macro(source: &str) -> BTreeSet<&'static str> {
    wasm_store_methods_declared_by_macro(source)
        .into_iter()
        .filter(|(_, attribute)| attribute.contains("canic_query(internal"))
        .map(|(method, _)| method)
        .collect()
}

fn wasm_store_methods_declared_by_macro(source: &str) -> Vec<(&'static str, String)> {
    let mut methods = Vec::new();

    let mut offset = 0;
    while let Some(method_offset) = source[offset..].find("async fn canic_wasm_store_") {
        let method_start = offset + method_offset + "async fn ".len();
        let method_end = source[method_start..]
            .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .map(|end| method_start + end)
            .expect("wasm-store method name must terminate");
        let method = &source[method_start..method_end];
        let prefix = &source[..method_start];
        let attribute_start = prefix
            .rfind("#[$crate::canic_")
            .unwrap_or_else(|| panic!("missing endpoint attribute for {method}"));
        let attribute_end = source[attribute_start..]
            .find(']')
            .map(|end| attribute_start + end + 1)
            .expect("endpoint attribute must terminate");
        methods.push((
            leak_test_str(method),
            source[attribute_start..attribute_end].to_string(),
        ));
        offset = method_end;
    }

    methods
}

fn protected_wasm_store_methods_in_did(did: &str) -> BTreeSet<&'static str> {
    did.lines()
        .filter(|line| line.contains("(CanicInternalCallEnvelopeV1)"))
        .filter_map(did_method_name)
        .filter(|method| method.starts_with("canic_wasm_store_"))
        .map(leak_test_str)
        .collect()
}

fn structural_wasm_store_queries_in_did(did: &str) -> BTreeSet<&'static str> {
    did.lines()
        .filter(|line| line.contains(" query"))
        .filter_map(did_method_name)
        .filter(|method| method.starts_with("canic_wasm_store_") && !method.contains("bootstrap"))
        .map(leak_test_str)
        .collect()
}

fn did_method_name(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let method_end = trimmed.find(" : ")?;
    Some(&trimmed[..method_end])
}

fn method_set(methods: &[&'static str]) -> BTreeSet<&'static str> {
    methods.iter().copied().collect()
}

fn read_workspace_file(relative: &str) -> String {
    fs::read_to_string(workspace_root().join(relative))
        .unwrap_or_else(|err| panic!("failed to read {relative}: {err}"))
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}
