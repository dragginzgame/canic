//! Validate compiled Wire declarations and preserve them across artifact transforms.
//!
//! This proves inventory, identity and budget parity, not Rust type-layout parity.

use canic_contracts::protocol::wire_declaration::{CompiledMethod, DeclarationError, MethodKind};
use ic_host_artifacts::wasm::WasmFacts;
use ic_host_tools::interface::{self, ExportedMethod, MethodMode};
use serde_json::Value;
use sha2_host::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// Exact artifact/declaration mismatch, distinct from codec generation failure.
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum WireArtifactError {
    #[error("invalid or oversized final Wasm for Wire schema admission")]
    Wasm,
    #[error(transparent)]
    Declaration(#[from] DeclarationError),
    #[error("duplicate Wire method name or identity")]
    Duplicate,
    #[error("Wire declaration differs from compiled export")]
    Export,
    #[error("Wire identity, mode or budget changed during artifact transformation")]
    Contract,
    #[error("invalid, unsupported or oversized Wire schema")]
    Schema,
}

/// Browser bindings and the SHA-256 of the exact admitted schema document bytes.
#[derive(Debug, Eq, PartialEq)]
pub struct WireBindings {
    pub schema_sha256: [u8; 32],
    pub typescript: String,
    /// Exact supplier/runtime sources retained by the native producer, never substituted by Host.
    pub browser_modules: BTreeMap<String, String>,
}

/// Read compiled declarations and require an exact corresponding IC export.
/// Other exports may be separately classified raw or external Candid methods.
pub fn declarations<'a>(
    facts: &WasmFacts<'a>,
) -> Result<BTreeMap<&'a str, CompiledMethod<'a>>, WireArtifactError> {
    let exports = interface::exported_methods(facts).collect::<BTreeSet<_>>();
    let mut methods = BTreeMap::new();
    let mut ids = BTreeSet::new();
    for section in &facts.custom_sections {
        if section.name != canic_contracts::protocol::wire_declaration::SECTION {
            continue;
        }
        let mut bytes = section.data;
        while !bytes.is_empty() {
            let method = CompiledMethod::decode(&mut bytes)?;
            if !exports.contains(&exported(method)) {
                return Err(WireArtifactError::Export);
            }
            if methods.insert(method.name, method).is_some() || !ids.insert(method.id) {
                return Err(WireArtifactError::Duplicate);
            }
        }
    }
    Ok(methods)
}

pub(super) const fn exported(method: CompiledMethod<'_>) -> ExportedMethod<'_> {
    ExportedMethod {
        name: method.name,
        mode: match method.kind {
            MethodKind::Query => MethodMode::Query,
            MethodKind::CompositeQuery => MethodMode::CompositeQuery,
            MethodKind::Update => MethodMode::Update,
        },
    }
}

pub(super) fn validate_transform(
    before: &WasmFacts<'_>,
    after: &WasmFacts<'_>,
) -> Result<(), WireArtifactError> {
    if declarations(before)? != declarations(after)? {
        return Err(WireArtifactError::Contract);
    }
    Ok(())
}

#[cfg(test)]
mod tests;

/// Admit a bounded native schema against final Wasm before generating browser bindings.
///
/// Wire owns type/reference validation and TypeScript generation. Canic checks
/// schema generation, export kind, exact identities and all codec allowances.
/// This does not prove target-specific payload layout or publish a release identity.
pub fn generate_wire_bindings(
    facts: &WasmFacts<'_>,
    bytes: &[u8],
    runtime_module: &str,
) -> Result<WireBindings, WireArtifactError> {
    if bytes.len() > crate::MAX_DOCUMENT_READ_BYTES {
        return Err(WireArtifactError::Schema);
    }
    let schema: Value = serde_json::from_slice(bytes).map_err(|_| WireArtifactError::Schema)?;
    if schema["schema_version"].as_u64() != Some(1) {
        return Err(WireArtifactError::Schema);
    }
    let methods = schema["methods"]
        .as_array()
        .ok_or(WireArtifactError::Schema)?;
    let compiled = declarations(facts)?;
    if methods.is_empty() || methods.len() != compiled.len() {
        return Err(WireArtifactError::Contract);
    }
    let mut names = BTreeSet::new();
    for method in methods {
        let name = method["name"].as_str().ok_or(WireArtifactError::Schema)?;
        if !names.insert(name) {
            return Err(WireArtifactError::Duplicate);
        }
        let declaration = compiled.get(name).ok_or(WireArtifactError::Contract)?;
        require_schema_contract(method, *declaration)?;
    }
    let types =
        serde_json::from_value(schema["types"].clone()).map_err(|_| WireArtifactError::Schema)?;
    let browser_modules = retained_browser_modules(&schema)?;
    let typescript = ic_wire::typescript::generate_registry(
        &types,
        methods,
        runtime_module,
        &std::collections::BTreeMap::new(),
        |report| eprint!("{report}"),
    )
    .map_err(|_| WireArtifactError::Schema)?;
    Ok(WireBindings {
        schema_sha256: Sha256::digest(bytes).into(),
        typescript,
        browser_modules,
    })
}

/// Require one exact method contract from an already release-admitted schema.
/// This selects a transport declaration, not payload layouts or release authority.
pub fn require_wire_method(
    bytes: &[u8],
    name: &str,
    id: u64,
    kind: MethodKind,
    request: ic_wire::Limits,
    response: ic_wire::Limits,
) -> Result<(), WireArtifactError> {
    if bytes.len() > crate::MAX_DOCUMENT_READ_BYTES {
        return Err(WireArtifactError::Schema);
    }
    let schema: Value = serde_json::from_slice(bytes).map_err(|_| WireArtifactError::Schema)?;
    if schema["schema_version"].as_u64() != Some(1) {
        return Err(WireArtifactError::Schema);
    }
    let methods = schema["methods"]
        .as_array()
        .ok_or(WireArtifactError::Schema)?;
    let identity = id.to_string();
    let mut selected = methods.iter().filter(|method| {
        method["name"].as_str() == Some(name) || method["id"].as_str() == Some(identity.as_str())
    });
    let method = selected.next().ok_or(WireArtifactError::Contract)?;
    if selected.next().is_some() {
        return Err(WireArtifactError::Duplicate);
    }
    if method["name"].as_str() != Some(name) {
        return Err(WireArtifactError::Contract);
    }
    let limits =
        |value: ic_wire::Limits| canic_contracts::protocol::wire_declaration::MethodLimits {
            bytes: value.bytes as u64,
            allocation: value.allocation as u64,
            nodes: value.nodes as u64,
            depth: value.depth as u64,
        };
    require_schema_contract(
        method,
        CompiledMethod {
            name,
            id,
            kind,
            request: limits(request),
            response: limits(response),
        },
    )
}

fn require_schema_contract(
    method: &Value,
    declaration: CompiledMethod<'_>,
) -> Result<(), WireArtifactError> {
    let (kind, mode) = match declaration.kind {
        MethodKind::Query => ("query", "query"),
        MethodKind::CompositeQuery => ("composite_query", "query"),
        MethodKind::Update => ("update", "update"),
    };
    let identity_matches = method["id"].as_str() == Some(declaration.id.to_string().as_str());
    let mode_matches =
        method["kind"].as_str() == Some(kind) && method["mode"].as_str() == Some(mode);
    let budgets_match = [
        ("request", declaration.request),
        ("response", declaration.response),
    ]
    .into_iter()
    .all(|(direction, limits)| {
        [
            ("Bytes", limits.bytes),
            ("Allocation", limits.allocation),
            ("Nodes", limits.nodes),
            ("Depth", limits.depth),
        ]
        .into_iter()
        .all(|(field, value)| method[format!("{direction}{field}")].as_u64() == Some(value))
    });
    if identity_matches && mode_matches && budgets_match {
        Ok(())
    } else {
        Err(WireArtifactError::Contract)
    }
}

fn retained_browser_modules(schema: &Value) -> Result<BTreeMap<String, String>, WireArtifactError> {
    let modules: BTreeMap<String, String> =
        serde_json::from_value(schema["browser_modules"].clone())
            .map_err(|_| WireArtifactError::Schema)?;
    let required = [
        "canic-adapter.ts",
        "identities.ts",
        "wire-adapter.ts",
        "wire-runtime.ts",
    ];
    if modules.keys().map(String::as_str).ne(required)
        || modules.values().any(|source| source.trim().is_empty())
    {
        return Err(WireArtifactError::Schema);
    }
    Ok(modules)
}
