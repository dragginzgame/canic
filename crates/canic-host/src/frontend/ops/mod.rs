//! Frontend source conversion, binding generation and bounded bundle I/O.
//!
//! These operations never provision canisters or publish external static assets.

mod bundle;
mod capacity;

use crate::{
    durable_io::read_regular_bytes,
    fleet_ensure::{policy::expected_plan_sha256, resolve_current_fleet},
    frontend::{
        FrontendError,
        model::{
            FrontendEnvironmentInput, FrontendFileRecord, FrontendManifestRecord,
            FrontendRoleRecord,
        },
        view::{FrontendAuthorityView, FrontendBindingsView, FrontendBundleView},
    },
    network::{frontend_root_key, resolve_canonical_network_id_from_root},
    protocol_binding::resolve_registry_protocol_binding,
};
use candid::TypeEnv;
use candid_parser::{
    IDLProg,
    bindings::{javascript, typescript},
    syntax::{Dec, IDLMergedProg},
};
use canic_core::cdk::utils::hash::{hex_bytes, sha256_hex};
use std::{collections::BTreeMap, path::Path};

pub use bundle::{manifest_digest, publish_bundle, verify_bundle};
pub use capacity::{asset_capacity, payload_inventory};

/// Maximum source/binding size per file, protecting host memory during export and verification.
pub const MAX_FRONTEND_FILE_BYTES: usize = 4 * 1024 * 1024;
/// Aggregate host bundle budget, independent of application database capacity.
pub const MAX_FRONTEND_BUNDLE_BYTES: usize = 32 * 1024 * 1024;

/// Decode only the bounded public input contract; unknown authority fields reject.
pub fn read_input(path: &Path) -> Result<FrontendEnvironmentInput, FrontendError> {
    Ok(serde_json::from_slice(&read_regular_bytes(
        path,
        MAX_FRONTEND_FILE_BYTES,
    )?)?)
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
            canic_core::dto::fleet_registry::FleetSubnetRootStatus::Active
                | canic_core::dto::fleet_registry::FleetSubnetRootStatus::Removed
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
    Ok(FrontendAuthorityView {
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

/// Produce browser bindings from a self-contained, type-checked Candid sidecar.
pub fn bindings(candid: &[u8]) -> Result<FrontendBindingsView, FrontendError> {
    if candid.len() > MAX_FRONTEND_FILE_BYTES {
        return Err(FrontendError::Bound("Candid bytes"));
    }
    let source =
        std::str::from_utf8(candid).map_err(|error| FrontendError::Candid(error.to_string()))?;
    let program: IDLProg = source
        .parse()
        .map_err(|error: candid_parser::Error| FrontendError::Candid(error.to_string()))?;
    if program
        .decs
        .iter()
        .any(|declaration| matches!(declaration, Dec::ImportType(_) | Dec::ImportServ(_)))
    {
        return Err(FrontendError::Candid(
            "sidecar imports are not release-bound".to_string(),
        ));
    }
    let mut environment = TypeEnv::new();
    let actor = candid_parser::check_prog(&mut environment, &program)
        .map_err(|error| FrontendError::Candid(error.to_string()))?;
    if actor.is_none() {
        return Err(FrontendError::Candid("missing service".to_string()));
    }
    let javascript = javascript::compile(&environment, &actor).into_bytes();
    let typescript =
        typescript::compile(&environment, &actor, &IDLMergedProg::new(program)).into_bytes();
    if javascript.len().max(typescript.len()) > MAX_FRONTEND_FILE_BYTES {
        return Err(FrontendError::Bound("generated binding bytes"));
    }
    Ok(FrontendBindingsView {
        javascript,
        typescript,
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
        let protocol = resolve_registry_protocol_binding(root, &input.environment, entry)?;
        let candid = read_regular_bytes(protocol.candid_path(), MAX_FRONTEND_FILE_BYTES)?;
        if sha256_hex(&candid) != hex_bytes(protocol.binding().candid_sha256) {
            return Err(FrontendError::Integrity);
        }
        let generated = bindings(&candid)?;
        let prefix = format!("roles/{}/{}", selected.role, selected.canister_id);
        roles.push(FrontendRoleRecord {
            role: selected.role.clone(),
            canister_id: selected.canister_id,
            release_identity: protocol.binding().release_identity.clone(),
            module_sha256: entry.module_hash.clone().ok_or(FrontendError::Integrity)?,
            candid: insert_file(&mut files, format!("{prefix}.did"), candid)?,
            javascript: insert_file(
                &mut files,
                format!("{prefix}.did.mjs"),
                generated.javascript,
            )?,
            typescript: insert_file(
                &mut files,
                format!("{prefix}.did.d.ts"),
                generated.typescript,
            )?,
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
        let observed = canic_core::ids::CanonicalNetworkId::from_der_root_trust_anchor(bytes)
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
