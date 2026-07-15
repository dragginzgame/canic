//! Module: replay_policy::tests
//!
//! Responsibility: verify replay-policy manifest coverage and release readiness.
//! Does not own: production replay policy data or workflow behavior.
//! Boundary: test-only checks comparing manifests to source-declared surfaces.

mod cost;
mod coverage;
mod endpoint;
mod pool_admin;
mod root_capability;

use super::{
    quota::{
        DEPLOYMENT_QUOTA_V1, DEPLOYMENT_RESERVE_V1, DURABLE_PUBLISH_QUOTA_V1,
        DURABLE_PUBLISH_RESERVE_V1, ISSUER_CANISTER_SIGNATURE_PREPARE_QUOTA_V1,
        ROOT_CANISTER_SIGNATURE_PREPARE_QUOTA_V1, VALUE_TRANSFER_QUOTA_V1,
        VALUE_TRANSFER_RESERVE_V1,
    },
    *,
};
use crate::protocol::{
    CANIC_TEMPLATE_PREPARE_ADMIN, CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
    CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN, CANIC_WASM_STORE_ROOT_UPDATE_METHODS,
};
use std::collections::BTreeSet;

fn pool_admin_command_variant_names() -> BTreeSet<&'static str> {
    enum_variant_names_from_source(
        include_str!("../../dto/pool.rs"),
        "pub enum PoolAdminCommand",
    )
}

fn root_capability_command_variant_names() -> BTreeSet<&'static str> {
    enum_variant_names_from_source(
        include_str!("../../workflow/rpc/request/handler/capability.rs"),
        "pub(in crate::workflow::rpc) enum RootCapability",
    )
}

fn durable_publish_endpoint_names() -> BTreeSet<&'static str> {
    std::iter::once("canic_wasm_store_admin").collect()
}

fn guarded_publication_effect_endpoint_names() -> BTreeSet<&'static str> {
    [
        CANIC_TEMPLATE_PREPARE_ADMIN,
        CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
        CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
    ]
    .into_iter()
    .chain(CANIC_WASM_STORE_ROOT_UPDATE_METHODS.iter().copied())
    .collect()
}

fn release_candidate_manifest_blockers() -> BTreeSet<String> {
    let endpoint_blockers = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .filter(|entry| entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker)
        .map(|entry| format!("endpoint:{}", entry.endpoint));

    let root_command_blockers = ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .filter(|entry| entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker)
        .map(|entry| format!("root-capability:{}", entry.variant));

    let pool_command_blockers = POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .filter(|entry| entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker)
        .map(|entry| format!("pool-admin:{}", entry.variant));

    endpoint_blockers
        .chain(root_command_blockers)
        .chain(pool_command_blockers)
        .collect()
}

const fn replay_command_kind(label: &'static str) -> ReplayCommandKindLabel {
    ReplayCommandKindLabel::new(label)
}

const fn replay_command_manifest(label: &'static str) -> ReplayCommandManifestLabel {
    ReplayCommandManifestLabel::new(label)
}

fn enum_variant_names_from_source(
    source: &'static str,
    marker: &'static str,
) -> BTreeSet<&'static str> {
    let start = source.find(marker).expect("enum exists in source");
    let body_start = source[start..]
        .find('{')
        .map(|offset| start + offset + 1)
        .expect("enum has body");
    let body_end = source[body_start..]
        .find("\n}")
        .map(|offset| body_start + offset)
        .expect("enum body closes");

    source[body_start..body_end]
        .lines()
        .filter_map(enum_variant_name_from_line)
        .collect()
}

fn enum_variant_name_from_line(line: &'static str) -> Option<&'static str> {
    let line = line.trim();
    let first = line.as_bytes().first().copied()?;
    if !first.is_ascii_uppercase() {
        return None;
    }
    let end = line
        .find(|ch: char| ch == '(' || ch == '{' || ch == ',' || ch.is_whitespace())
        .unwrap_or(line.len());
    Some(&line[..end])
}

fn emitted_update_endpoint_names() -> BTreeSet<&'static str> {
    [
        include_str!("../../../../canic/src/macros/endpoints/root.rs"),
        include_str!("../../../../canic/src/macros/endpoints/shared.rs"),
        include_str!("../../../../canic/src/macros/endpoints/wasm_store.rs"),
        include_str!("../../../../canic/src/macros/endpoints/nonroot.rs"),
        include_str!("../../../../canic/src/macros/endpoints/icp_refill.rs"),
    ]
    .into_iter()
    .flat_map(update_endpoint_names_from_source)
    .collect()
}

fn update_endpoint_names_from_source(source: &'static str) -> Vec<&'static str> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut names = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if !line.contains("#[$crate::canic_update") {
            continue;
        }
        let Some(name) = lines
            .iter()
            .skip(index + 1)
            .take(6)
            .find_map(|candidate| endpoint_name_from_fn_line(candidate))
        else {
            panic!("canic_update endpoint attribute without following function");
        };
        names.push(name);
    }
    names
}

fn endpoint_name_from_fn_line(line: &'static str) -> Option<&'static str> {
    let marker = "fn ";
    let start = line.find(marker)? + marker.len();
    let rest = &line[start..];
    let end = rest.find('(')?;
    Some(&rest[..end])
}
