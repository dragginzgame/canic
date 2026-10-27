//! Native endpoint-derived Wire metadata. Wire owns schema and codec validation.
//!
//! Final compiled identities must match before the native projection is returned.
//! Matching identities does not establish target-specific Rust type-layout parity.

pub mod producer;

use canic_contracts::ids::wire_schema;
use ic_wire::{
    Limits,
    typescript::{self, GenerationError, Wire},
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
};

pub use canic_contracts::protocol::wire_declaration::{CompiledMethod, MethodKind, MethodLimits};

/// A compiled method and the actual Serde layouts of its request and reply.
pub struct Method {
    declaration: CompiledMethod<'static>,
    request: Value,
    response: Value,
}

impl Method {
    /// Derive metadata from the same signature and budgets as the entrypoint.
    pub fn describe<Request: Wire, Response: Wire>(
        name: &'static str,
        kind: MethodKind,
        id: u64,
        request_limits: Limits,
        response_limits: Limits,
    ) -> Result<Self, GenerationError> {
        let limits = |value: Limits| MethodLimits {
            bytes: value.bytes as u64,
            allocation: value.allocation as u64,
            nodes: value.nodes as u64,
            depth: value.depth as u64,
        };
        Ok(Self::project(
            CompiledMethod {
                name,
                kind,
                id,
                request: limits(request_limits),
                response: limits(response_limits),
            },
            typescript::native_schema::<Request>()?,
            typescript::native_schema::<Response>()?,
        ))
    }

    /// Import actual payload projections from a supplier owning a raw endpoint codec.
    /// Snapshot admission validates these schemas and the complete compiled contract.
    /// Do not infer projections from the raw handler signature or from Candid.
    #[must_use]
    pub const fn project(
        declaration: CompiledMethod<'static>,
        request: Value,
        response: Value,
    ) -> Self {
        Self {
            declaration,
            request,
            response,
        }
    }

    const fn declaration(&self) -> CompiledMethod<'static> {
        self.declaration
    }

    fn metadata(&self) -> Value {
        let (kind, mode) = match self.declaration.kind {
            MethodKind::Query => ("query", "query"),
            MethodKind::CompositeQuery => ("composite_query", "query"),
            MethodKind::Update => ("update", "update"),
        };
        json!({
            "name": self.declaration.name, "kind": kind, "mode": mode, "id": self.declaration.id.to_string(),
            "request": self.request, "response": self.response,
            "requestBytes": self.declaration.request.bytes,
            "requestAllocation": self.declaration.request.allocation,
            "requestNodes": self.declaration.request.nodes,
            "requestDepth": self.declaration.request.depth,
            "responseBytes": self.declaration.response.bytes,
            "responseAllocation": self.declaration.response.allocation,
            "responseNodes": self.declaration.response.nodes,
            "responseDepth": self.declaration.response.depth,
        })
    }
}

/// Deferred projection of the methods contributed by one mounted declaration owner.
type Projection = fn() -> Result<Vec<Method>, GenerationError>;

/// One mounted owner and its optional, explicitly composed browser adapter.
#[derive(Clone, Copy)]
struct Provider {
    methods: Projection,
    browser_adapter: Option<fn(&str) -> String>,
}

static PROVIDERS: Mutex<Vec<Provider>> = Mutex::new(Vec::new());

/// Constructor hook for generated declarations; generation runs outside the lock.
///
/// # Panics
/// Panics if a previous registration poisoned the registry lock.
#[doc(hidden)]
pub fn register(methods: Projection, browser_adapter: Option<fn(&str) -> String>) {
    PROVIDERS
        .lock()
        .expect("Wire declaration registry")
        .push(Provider {
            methods,
            browser_adapter,
        });
}

/// Read one deterministic schema from every compiled typed Wire declaration.
///
/// Require the declarations read from the final Wasm, not a reconstructed inventory.
/// Reject empty/duplicate inventories, identity/budget drift and unsupported projections.
/// The caller owns Wasm export admission and target-specific type-layout qualification.
pub fn snapshot(compiled: &[CompiledMethod<'_>]) -> Result<Value, GenerationError> {
    let providers = PROVIDERS
        .lock()
        .map_err(|_| GenerationError::Schema)?
        .clone();
    project(providers, Some(compiled))
}

/// Produce the native contract before runtime compilation.
///
/// This validates the registered layouts but does not admit a Wasm artifact.
/// Delivery must still call `snapshot` with final compiled declarations.
pub fn native_snapshot() -> Result<Value, GenerationError> {
    let providers = PROVIDERS
        .lock()
        .map_err(|_| GenerationError::Schema)?
        .clone();
    project(providers, None)
}

fn project(
    providers: Vec<Provider>,
    compiled: Option<&[CompiledMethod<'_>]>,
) -> Result<Value, GenerationError> {
    let mut adapters = providers
        .iter()
        .filter_map(|provider| provider.browser_adapter);
    let adapter = adapters.next();
    // Multiple suppliers must compose explicitly; registration order is not authority.
    if adapters.next().is_some() {
        return Err(GenerationError::Schema);
    }
    let methods: Vec<_> = providers
        .into_iter()
        .map(|provider| (provider.methods)())
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();
    let native: Vec<_> = methods.iter().map(Method::declaration).collect();
    let mut document = assemble(methods, compiled.unwrap_or(&native))?;
    document["browser_modules"] =
        serde_json::to_value(browser_modules(adapter)).map_err(|_| GenerationError::Schema)?;
    Ok(document)
}

fn browser_modules(adapter: Option<fn(&str) -> String>) -> BTreeMap<&'static str, String> {
    BTreeMap::from([
        ("wire-runtime.ts", typescript::RUNTIME.to_owned()),
        (
            "identities.ts",
            wire_schema::TYPESCRIPT_VALIDATOR.to_owned(),
        ),
        (
            "canic-adapter.ts",
            wire_schema::TYPESCRIPT_ADAPTER.to_owned(),
        ),
        (
            "wire-adapter.ts",
            adapter.map_or_else(
                || "export { createCodec, WireError } from \"./canic-adapter.js\";\n".to_owned(),
                |adapter| adapter("./canic-adapter.js"),
            ),
        ),
    ])
}

fn assemble(
    mut methods: Vec<Method>,
    compiled: &[CompiledMethod<'_>],
) -> Result<Value, GenerationError> {
    if methods.is_empty() {
        return Err(GenerationError::Schema);
    }
    methods.sort_by_key(|method| method.declaration.name);
    let mut ids = BTreeSet::new();
    if methods
        .iter()
        .any(|method| !ids.insert(method.declaration.id))
    {
        return Err(GenerationError::Schema);
    }
    let expected = methods
        .iter()
        .map(|method| (method.declaration.name, method.declaration()))
        .collect::<BTreeMap<_, _>>();
    let actual = compiled
        .iter()
        .map(|method| (method.name, *method))
        .collect::<BTreeMap<_, _>>();
    if actual.len() != compiled.len() || expected != actual {
        return Err(GenerationError::Schema);
    }
    let mut types = BTreeMap::new();
    let mut metadata: Vec<_> = methods.iter().map(Method::metadata).collect();
    for method in &mut metadata {
        for direction in ["request", "response"] {
            let name = format!(
                "{}_{}",
                method["name"].as_str().ok_or(GenerationError::Schema)?,
                direction
            );
            let schema = method[direction].take();
            if types.insert(name.clone(), schema).is_some() {
                return Err(GenerationError::Schema);
            }
            method[direction] = json!({"k": "ref", "name": name});
        }
    }
    // Delegate names, duplicate names, schema references and codec budgets.
    typescript::generate_registry(
        &types,
        &metadata,
        "./wire-adapter.js",
        &std::collections::BTreeMap::new(),
        |report| eprint!("{report}"),
    )?;
    Ok(json!({"schema_version": 1, "types": types, "methods": metadata}))
}

#[cfg(test)]
mod tests;
