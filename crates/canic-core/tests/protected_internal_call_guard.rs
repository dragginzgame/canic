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
    let protected_methods = protected_internal_method_tokens();
    let mut violations = Vec::new();

    for root in FIRST_PARTY_SRC_ROOTS {
        scan_dir(&workspace.join(root), &protected_methods, &mut violations);
    }

    assert!(
        violations.is_empty(),
        "protected Canic internal methods must be called through CanicCall: {violations:#?}"
    );
}

#[test]
fn raw_call_guard_catches_multiline_protected_method_literals() {
    let protected_methods = protected_internal_method_tokens();
    let source = r#"
async fn wrong(pid: Principal) {
    let _ = CallOps::unbounded_wait(
        pid,
        "canic_wasm_store_prepare",
    )
    .execute()
    .await;
}
"#;

    let violations = raw_call_violations_in_source(
        Path::new("crates/example/src/lib.rs"),
        source,
        &protected_methods,
    );

    assert_eq!(violations, vec!["crates/example/src/lib.rs:3".to_string()]);
}

#[test]
fn raw_call_guard_catches_multiline_protected_method_constants() {
    let protected_methods = protected_internal_method_tokens();
    let source = r"
async fn wrong(pid: Principal) {
    let _ = Call::unbounded_wait(
        pid,
        CANIC_WASM_STORE_PREPARE,
    )
    .execute()
    .await;
}
";

    let violations = raw_call_violations_in_source(
        Path::new("crates/example/src/lib.rs"),
        source,
        &protected_methods,
    );

    assert_eq!(violations, vec!["crates/example/src/lib.rs:3".to_string()]);
}

#[test]
fn raw_call_guard_allows_multiline_external_or_structural_methods() {
    let protected_methods = protected_internal_method_tokens();
    let source = r#"
async fn allowed(pid: Principal) {
    let _ = Call::unbounded_wait(
        pid,
        "icrc1_balance_of",
    )
    .execute()
    .await;

    let _ = CallOps::unbounded_wait(
        pid,
        "canic_wasm_store_catalog",
    )
    .execute()
    .await;
}
"#;

    let violations = raw_call_violations_in_source(
        Path::new("crates/example/src/lib.rs"),
        source,
        &protected_methods,
    );

    assert!(violations.is_empty(), "{violations:#?}");
}

#[test]
fn raw_call_guard_does_not_treat_canic_call_as_raw_call() {
    let protected_methods = protected_internal_method_tokens();
    let source = r#"
async fn allowed(pid: Principal) {
    let _ = CanicCall::unbounded_wait(
        pid,
        "canic_wasm_store_prepare",
    )
    .execute()
    .await;
}
"#;

    let violations = raw_call_violations_in_source(
        Path::new("crates/example/src/lib.rs"),
        source,
        &protected_methods,
    );

    assert!(violations.is_empty(), "{violations:#?}");
}

#[test]
fn protected_method_discovery_handles_nested_role_arrays() {
    let source = r#"
#[canic_update(
    internal,
    name = "wire_multi_role_endpoint",
    requires(caller::has_any_role([
        "project_hub",
        "admin_hub",
    ]))
)]
async fn multi_role_endpoint() -> Result<(), canic::Error> {
    Ok(())
}
"#;
    let mut methods = BTreeSet::new();

    collect_protected_update_methods(source, &mut methods);

    assert!(methods.contains("wire_multi_role_endpoint"), "{methods:#?}");
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
            line.contains(" : () -> "),
            "{method} must expose a no-argument raw-ingress wrapper ABI: {line}"
        );
        assert!(
            !line.contains(" query"),
            "{method} must be an update method in the protected raw-ingress ABI: {line}"
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

#[test]
fn project_fixture_uses_shared_descriptor_generated_client_path() {
    let protocol = read_workspace_file("canisters/test/project_protocol_stub/src/lib.rs");
    let instance = read_workspace_file("canisters/test/project_instance_stub/src/lib.rs");
    let hub = read_workspace_file("canisters/test/project_hub_stub/src/lib.rs");

    assert!(
        protocol.contains("canic::canic_protected_endpoint!"),
        "project fixture must publish protected endpoint metadata from a shared protocol crate"
    );
    assert!(
        protocol.contains("project_instance_record_visit_endpoint"),
        "project fixture must expose a shared descriptor for the instance endpoint"
    );
    assert!(
        instance.contains("name = \"project_instance_record_visit\"")
            && instance.contains("requires(caller::has_role(\"project_hub\"))"),
        "project instance fixture endpoint must be protected by project_hub role proof"
    );
    assert!(
        hub.contains("canic::canic_internal_client!")
            && hub.contains("project_instance_record_visit_endpoint")
            && hub.contains(".record_visit(project_key)"),
        "project hub fixture must call the instance through the generated protected client"
    );
    assert!(
        !hub.contains("ProtectedInternalEndpoint::new")
            && !hub.contains("CanicCall::")
            && !hub.contains("Call::"),
        "project hub fixture must not hand-build protected metadata or bypass the generated client"
    );
}

#[test]
fn protected_internal_method_guard_includes_shared_protocol_descriptors() {
    let protected_methods = protected_internal_method_names();

    assert!(
        protected_methods.contains("project_instance_record_visit"),
        "shared protocol descriptors must be included in the raw-call guard method set"
    );
}

#[test]
fn canic_call_dispatches_protected_envelope_as_raw_ingress_bytes() {
    let source = read_workspace_file("crates/canic-core/src/api/ic/call.rs");

    assert!(
        source.contains(".with_raw_args(encode_internal_call_envelope_raw(envelope)?)"),
        "CanicCall must send protected envelopes through the raw-args boundary"
    );
    assert!(
        !source.contains(".with_arg(envelope)?"),
        "CanicCall must not expose the protected envelope as a typed Candid argument"
    );
}

fn scan_dir(root: &Path, protected_methods: &BTreeSet<String>, violations: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir(&path, protected_methods, violations);
            continue;
        }

        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }

        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };

        violations.extend(raw_call_violations_in_source(
            &path,
            &contents,
            protected_methods,
        ));
    }
}

fn raw_call_violations_in_source(
    path: &Path,
    source: &str,
    protected_methods: &BTreeSet<String>,
) -> Vec<String> {
    if is_allowed_boundary(path) {
        return Vec::new();
    }

    let mut violations = Vec::new();
    let mut offset = 0;
    while let Some((call_start, _pattern)) = next_raw_call_offset(&source[offset..]) {
        let absolute = offset + call_start;
        let window = raw_call_expression_window(&source[absolute..]);
        if protected_methods
            .iter()
            .any(|method| window.contains(method.as_str()))
        {
            violations.push(format!(
                "{}:{}",
                path.display(),
                line_number(source, absolute)
            ));
        }
        offset = absolute + 1;
    }
    violations
}

fn next_raw_call_offset(source: &str) -> Option<(usize, &'static str)> {
    RAW_CALL_PATTERNS
        .iter()
        .filter_map(|pattern| next_pattern_offset(source, pattern).map(|offset| (offset, *pattern)))
        .min_by_key(|(offset, _)| *offset)
}

fn next_pattern_offset(source: &str, pattern: &str) -> Option<usize> {
    let mut offset = 0;
    while let Some(found) = source[offset..].find(pattern) {
        let absolute = offset + found;
        if is_real_raw_call_start(source, absolute) {
            return Some(absolute);
        }
        offset = absolute + pattern.len();
    }
    None
}

fn is_real_raw_call_start(source: &str, offset: usize) -> bool {
    source[..offset]
        .chars()
        .next_back()
        .is_none_or(|ch| !(ch.is_ascii_alphanumeric() || ch == '_'))
}

fn raw_call_expression_window(source: &str) -> &str {
    let end = source
        .char_indices()
        .find_map(|(index, ch)| (ch == ';').then_some(index + ch.len_utf8()))
        .unwrap_or_else(|| source.len().min(2_000));
    &source[..end.min(source.len()).min(2_000)]
}

fn line_number(source: &str, offset: usize) -> usize {
    source[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn protected_internal_method_tokens() -> BTreeSet<String> {
    protected_internal_method_names()
        .into_iter()
        .flat_map(|method| {
            [
                method.clone(),
                method
                    .chars()
                    .map(|ch| {
                        if ch.is_ascii_alphanumeric() {
                            ch.to_ascii_uppercase()
                        } else {
                            '_'
                        }
                    })
                    .collect(),
            ]
        })
        .collect()
}

fn protected_internal_method_names() -> BTreeSet<String> {
    let workspace = workspace_root();
    let mut methods = CANIC_WASM_STORE_PROTECTED_UPDATE_METHODS
        .iter()
        .map(|method| (*method).to_string())
        .collect::<BTreeSet<_>>();

    for root in FIRST_PARTY_SRC_ROOTS {
        collect_protected_methods_from_dir(&workspace.join(root), &mut methods);
    }

    methods
}

fn collect_protected_methods_from_dir(root: &Path, methods: &mut BTreeSet<String>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_protected_methods_from_dir(&path, methods);
            continue;
        }

        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }

        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };

        collect_shared_descriptor_methods(&source, methods);
        collect_protected_update_methods(&source, methods);
    }
}

fn collect_shared_descriptor_methods(source: &str, methods: &mut BTreeSet<String>) {
    let mut offset = 0;
    while let Some(marker_offset) = source[offset..].find("canic_protected_endpoint!") {
        let marker_start = offset + marker_offset;
        if marker_is_macro_definition(source, marker_start) {
            offset = marker_start + "canic_protected_endpoint!".len();
            continue;
        }
        let Some(open_offset) = source[marker_start..].find('{') else {
            break;
        };
        let open = marker_start + open_offset;
        let Some(close) = matching_brace(source, open) else {
            break;
        };
        let body = &source[(open + 1)..close];

        for statement in body.split(';') {
            let Some(fn_offset) = statement.find("fn ") else {
                continue;
            };
            let Some(equals_offset) = statement[fn_offset..].find('=') else {
                continue;
            };
            let after_equals = &statement[(fn_offset + equals_offset + 1)..];
            if let Some(method) = leading_string_literal(after_equals) {
                methods.insert(method.to_string());
            }
        }

        offset = close + 1;
    }
}

fn collect_protected_update_methods(source: &str, methods: &mut BTreeSet<String>) {
    let mut offset = 0;
    while let Some(attribute_offset) = next_canic_update_attribute_offset(&source[offset..]) {
        let attribute_start = offset + attribute_offset;
        let Some(attribute_end) = attribute_end(source, attribute_start) else {
            break;
        };
        let attribute = &source[attribute_start..attribute_end];
        offset = attribute_end;

        if !attribute.contains("internal")
            || !(attribute.contains("caller::has_role")
                || attribute.contains("caller::has_any_role"))
        {
            continue;
        }

        if let Some(method) = protected_endpoint_method_name(attribute, &source[attribute_end..]) {
            methods.insert(method.to_string());
        }
    }
}

fn next_canic_update_attribute_offset(source: &str) -> Option<usize> {
    ["#[canic_update", "#[$crate::canic_update"]
        .into_iter()
        .filter_map(|needle| {
            let mut offset = 0;
            while let Some(attribute_offset) = source[offset..].find(needle) {
                let absolute = offset + attribute_offset;
                if is_real_attribute_start(source, absolute) {
                    return Some(absolute);
                }
                offset = absolute + needle.len();
            }
            None
        })
        .min()
}

fn marker_is_macro_definition(source: &str, marker_start: usize) -> bool {
    source[..marker_start]
        .rsplit('\n')
        .next()
        .is_some_and(|line| line.contains("macro_rules!"))
}

fn is_real_attribute_start(source: &str, attribute_start: usize) -> bool {
    source[..attribute_start]
        .chars()
        .next_back()
        .is_none_or(char::is_whitespace)
}

fn protected_endpoint_method_name<'a>(
    attribute: &'a str,
    after_attribute: &'a str,
) -> Option<&'a str> {
    if let Some(name_offset) = attribute.find("name") {
        let after_name = &attribute[(name_offset + "name".len())..];
        let equals_offset = after_name.find('=')?;
        if let Some(method) = leading_string_literal(&after_name[(equals_offset + 1)..]) {
            return Some(method);
        }
    }

    next_function_name(after_attribute)
}

fn matching_brace(source: &str, open: usize) -> Option<usize> {
    let mut depth = 0_u32;
    for (relative, ch) in source[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(open + relative);
                }
            }
            _ => {}
        }
    }
    None
}

fn leading_string_literal(source: &str) -> Option<&str> {
    let trimmed = source.trim_start();
    let rest = trimmed.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(&rest[..end])
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
        let Some(attribute_end) = attribute_end(source, attribute_start) else {
            break;
        };
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
    let attribute_end =
        attribute_end(source, attribute_start).expect("endpoint attribute must terminate");
    &source[attribute_start..attribute_end.min(function_start)]
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
        let attribute_end =
            attribute_end(source, attribute_start).expect("endpoint attribute must terminate");
        methods.push((
            leak_test_str(method),
            source[attribute_start..attribute_end].to_string(),
        ));
        offset = method_end;
    }

    methods
}

fn attribute_end(source: &str, attribute_start: usize) -> Option<usize> {
    let open = source[attribute_start..]
        .find('[')
        .map(|offset| attribute_start + offset)?;
    matching_bracket(source, open).map(|close| close + 1)
}

fn matching_bracket(source: &str, open: usize) -> Option<usize> {
    let mut depth = 0_u32;
    for (relative, ch) in source[open..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(open + relative);
                }
            }
            _ => {}
        }
    }
    None
}

fn protected_wasm_store_methods_in_did(did: &str) -> BTreeSet<&'static str> {
    did.lines()
        .filter(|line| !line.contains(" query"))
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
