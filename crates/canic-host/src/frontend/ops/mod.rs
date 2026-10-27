//! Frontend source conversion, binding generation and bounded bundle I/O.
//!
//! These operations never provision canisters or publish external static assets.

mod bundle;
mod capacity;
pub mod compiler;
mod uploaded;
mod validation;

use crate::{
    fleet_ensure::{policy::expected_plan_sha256, resolve_current_fleet},
    frontend::{
        FrontendError,
        model::{
            FrontendEnvironmentInput, FrontendFileRecord, FrontendManifestRecord,
            FrontendRoleRecord,
        },
        view::{FrontendAuthorityView, FrontendBundleView},
    },
    network::{frontend_root_key, resolve_canonical_network_id_from_root},
    protocol_binding::resolve_release_registry_wire_schema,
};
use canic_core::cdk::utils::hash::{hex_bytes, sha256_hex};
use ic_host_fs::read::read_file_no_follow;
use std::{collections::BTreeMap, path::Path};

pub use bundle::{manifest_digest, publish_bundle, verify_bundle};
pub use capacity::{asset_capacity, payload_inventory};
pub use uploaded::{FrontendAssetReader, IcpFrontendAssetReader, prepare_uploaded};

/// Maximum source/binding size per file, protecting host memory during export and verification.
pub const MAX_FRONTEND_FILE_BYTES: usize = 4 * 1024 * 1024;
/// Aggregate host bundle budget, independent of application database capacity.
pub const MAX_FRONTEND_BUNDLE_BYTES: usize = 32 * 1024 * 1024;

/// Decode only the bounded public input contract; unknown authority fields reject.
pub fn read_input(path: &Path) -> Result<FrontendEnvironmentInput, FrontendError> {
    Ok(serde_json::from_slice(
        &read_file_no_follow(path, MAX_FRONTEND_FILE_BYTES).map_err(std::io::Error::from)?,
    )?)
}

/// Project a verified terminal review and its selected network into export facts.
pub fn authority(
    root: &Path,
    environment: &str,
    fleet: &str,
) -> Result<FrontendAuthorityView, FrontendError> {
    let current = resolve_current_fleet(root, environment, fleet)?;
    if expected_plan_sha256(&current.plan) != current.plan.plan_sha256 {
        return Err(FrontendError::Integrity);
    }
    let registry = current.initial_active_registry(fleet)?;
    if registry.fleet_subnet_roots.iter().any(|root| {
        !matches!(
            root.status,
            canic_contracts::dto::fleet_registry::FleetSubnetRootStatus::Active
                | canic_contracts::dto::fleet_registry::FleetSubnetRootStatus::Removed
        )
    }) {
        return Err(FrontendError::Integrity);
    }
    let network = resolve_canonical_network_id_from_root(root, environment)?;
    if registry.authority.binding.fleet.fleet.canonical_network_id != network {
        return Err(FrontendError::Environment);
    }
    let admission_origin = current
        .plan
        .reviewed_desired
        .as_deref()
        .and_then(|review| review.desired().bootstrap.as_ref())
        .and_then(|bootstrap| bootstrap.admission_identity_origin.clone());
    let release_build_id = current
        .plan
        .reviewed_desired
        .as_ref()
        .and_then(|review| review.desired().bootstrap.as_ref())
        .ok_or(FrontendError::Integrity)?
        .release_build_id;
    Ok(FrontendAuthorityView {
        release_build_id,
        app: registry.authority.binding.fleet.app.to_string(),
        fleet: fleet.to_string(),
        fleet_id: registry.authority.binding.fleet.fleet.fleet_id.to_string(),
        source_plan_sha256: current.plan.plan_sha256.clone(),
        network,
        admission_nonempty: !registry.admission.fleet_principals.is_empty()
            || registry
                .admission
                .rules
                .iter()
                .any(|rule| !rule.principals.is_empty()),
        admission_origin,
        entries: current.registry.entries,
    })
}

fn insert_file(
    files: &mut BTreeMap<String, Vec<u8>>,
    path: String,
    bytes: Vec<u8>,
) -> Result<FrontendFileRecord, FrontendError> {
    let total = files.values().map(Vec::len).sum::<usize>();
    if bytes.len() > MAX_FRONTEND_FILE_BYTES
        || total.saturating_add(bytes.len()) > MAX_FRONTEND_BUNDLE_BYTES
    {
        return Err(FrontendError::Bound("bundle bytes"));
    }
    let record = FrontendFileRecord {
        path: path.clone(),
        sha256: sha256_hex(&bytes),
        bytes: bytes.len() as u64,
    };
    if files.insert(path, bytes).is_some() {
        return Err(FrontendError::Integrity);
    }
    Ok(record)
}

/// Convert selected terminal roles into an immutable public manifest and exact file bytes.
pub fn prepare_bundle(
    root: &Path,
    input: &FrontendEnvironmentInput,
    authority: FrontendAuthorityView,
    compiler: &compiler::BrowserCompiler,
) -> Result<FrontendBundleView, FrontendError> {
    let mut files = BTreeMap::new();
    let mut roles = Vec::new();
    let mut selections = input.roles.iter().collect::<Vec<_>>();
    selections.sort_by_key(|selection| (selection.role.as_str(), selection.canister_id));
    for selected in selections {
        let matches = authority
            .entries
            .iter()
            .filter(|entry| {
                entry.pid == selected.canister_id.to_text()
                    && entry.role.as_deref() == Some(selected.role.as_str())
            })
            .collect::<Vec<_>>();
        let [entry] = matches.as_slice() else {
            return Err(FrontendError::Role(selected.role.to_string()));
        };
        let built = resolve_release_registry_wire_schema(root, authority.release_build_id, entry)?;
        let generated = compile_role(root, compiler, &built, selected.role.as_str())?;
        let prefix = format!("roles/{}/{}", selected.role, selected.canister_id);
        let compiler_identity = generated.acquisition.record().key().to_string();
        let mut retained = BTreeMap::new();
        for (name, bytes) in generated.files {
            retained.insert(
                name.clone(),
                insert_file(&mut files, format!("{prefix}/{name}"), bytes.into_bytes())?,
            );
        }
        roles.push(FrontendRoleRecord {
            role: selected.role.clone(),
            canister_id: selected.canister_id,
            release_identity: built.release_build_id.to_string(),
            module_sha256: built.wasm_sha256,
            wire_schema: insert_file(&mut files, format!("{prefix}/schema.json"), built.schema)?,
            compiler_identity,
            javascript: retained
                .remove("bindings.js")
                .ok_or(FrontendError::Integrity)?,
            typescript: retained
                .remove("bindings.d.ts")
                .ok_or(FrontendError::Integrity)?,
            modules: retained.into_values().collect(),
        });
    }
    let alternative_origins = insert_file(
        &mut files,
        ".well-known/ii-alternative-origins".to_string(),
        serde_json::to_vec(
            &serde_json::json!({"alternativeOrigins": input.identity.alternative_origins}),
        )?,
    )?;
    let root_key = frontend_root_key(root, &input.environment)?;
    if let Some(bytes) = &root_key {
        let observed = canic_contracts::ids::CanonicalNetworkId::from_der_root_trust_anchor(bytes)
            .map_err(|_| FrontendError::Integrity)?;
        if observed != authority.network {
            return Err(FrontendError::Environment);
        }
    }
    let mut manifest = FrontendManifestRecord {
        schema_version: 1,
        manifest_sha256: String::new(),
        generator: format!("canic-host/{}", env!("CARGO_PKG_VERSION")),
        app: authority.app,
        fleet: authority.fleet,
        fleet_id: authority.fleet_id,
        source_plan_sha256: authority.source_plan_sha256,
        environment: input.environment.clone(),
        canonical_network_id: authority.network,
        api_origin: input.api_origin.clone(),
        local_root_key_der_hex: root_key.map(hex_bytes),
        identity: input.identity.clone(),
        alternative_origins,
        asset: input.asset.clone(),
        roles,
    };
    manifest.manifest_sha256 = manifest_digest(&manifest)?;
    Ok(FrontendBundleView { manifest, files })
}

fn compile_role(
    root: &Path,
    compiler: &compiler::BrowserCompiler,
    built: &crate::release_set::BuiltWireSchema,
    role: &str,
) -> Result<compiler::CompiledBrowserBindings, FrontendError> {
    let wasm = read_file_no_follow(
        &built.path.with_extension("").with_extension("wasm"),
        crate::MAX_ARTIFACT_READ_BYTES,
    )
    .map_err(std::io::Error::from)?;
    if sha256_hex(&wasm) != built.wasm_sha256 {
        return Err(FrontendError::Integrity);
    }
    let facts = ic_host_artifacts::wasm::inspect(
        &wasm,
        ic_host_artifacts::wasm::InspectionLimits {
            module_bytes: crate::MAX_ARTIFACT_READ_BYTES,
            sections: 1024,
            exports: 4096,
            custom_sections: 1024,
        },
    )
    .map_err(|_| FrontendError::Integrity)?;
    let bindings =
        crate::artifact_io::generate_wire_bindings(&facts, &built.schema, "./wire-adapter.js")?;
    Ok(compiler::compile(
        &root.join(".canic/cache/browser"),
        &root
            .join(".canic/frontend-builds")
            .join(built.release_build_id.to_string())
            .join(format!("{role}.json")),
        compiler,
        &bindings,
    )?)
}
