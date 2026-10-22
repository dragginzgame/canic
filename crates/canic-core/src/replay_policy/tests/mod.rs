//! Module: replay_policy::tests
//!
//! Responsibility: verify replay-policy manifest coverage and release readiness.
//! Does not own: production replay policy data or workflow behavior.
//! Boundary: test-only checks comparing manifests to source-declared surfaces.

mod cost;
mod coverage;
mod endpoint;
mod role_command;
mod root_capability;

use super::{
    quota::{
        DURABLE_PUBLISH_QUOTA_V1, DURABLE_PUBLISH_RESERVE_V1, VALUE_TRANSFER_QUOTA_V1,
        VALUE_TRANSFER_RESERVE_V1,
    },
    *,
};
use std::collections::BTreeSet;

fn root_capability_command_variant_names() -> BTreeSet<&'static str> {
    enum_variant_names_from_source(
        include_str!("../../workflow/rpc/request/handler/capability.rs"),
        "pub(in crate::workflow::rpc) enum RootCapability",
    )
}

fn release_candidate_manifest_blockers() -> BTreeSet<String> {
    let endpoint_blockers = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .chain(STORE_ENDPOINT_REPLAY_POLICY_MANIFEST)
        .filter(|entry| entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker)
        .map(|entry| format!("endpoint:{}", entry.endpoint));

    let root_command_blockers = ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .filter(|entry| entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker)
        .map(|entry| format!("root-capability:{}", entry.variant));

    let root_role_blockers = ROOT_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .filter(|entry| entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker)
        .map(|entry| format!("root-command:{}", entry.variant));

    let coordinator_role_blockers = COORDINATOR_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .filter(|entry| entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker)
        .map(|entry| format!("coordinator-command:{}", entry.variant));

    let managed_role_blockers = MANAGED_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .filter(|entry| entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker)
        .map(|entry| format!("managed-command:{}", entry.variant));

    let store_role_blockers = STORE_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .filter(|entry| entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker)
        .map(|entry| format!("store-command:{}", entry.variant));

    endpoint_blockers
        .chain(root_command_blockers)
        .chain(root_role_blockers)
        .chain(coordinator_role_blockers)
        .chain(managed_role_blockers)
        .chain(store_role_blockers)
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
    enum_variant_names_from_source_until(source, marker, "\n}")
}

fn enum_variant_names_from_source_until(
    source: &'static str,
    marker: &'static str,
    closing: &'static str,
) -> BTreeSet<&'static str> {
    let start = source
        .match_indices(marker)
        .find(|(offset, _)| {
            source[offset + marker.len()..]
                .trim_start()
                .starts_with('{')
        })
        .map(|(offset, _)| offset)
        .expect("exact enum exists in source");
    let body_start = source[start..]
        .find('{')
        .map(|offset| start + offset + 1)
        .expect("enum has body");
    let body_end = source[body_start..]
        .find(closing)
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

fn root_command_variant_names() -> BTreeSet<&'static str> {
    enum_variant_names_from_source_until(
        include_str!("../../../../canic-contracts/src/dto/wire/root_command/mod.rs"),
        "pub enum RootCommand",
        "\n        }",
    )
}

fn coordinator_command_variant_names() -> BTreeSet<&'static str> {
    enum_variant_names_from_source(
        include_str!("../../../../canic-contracts/src/dto/fleet_coordinator/mod.rs"),
        "pub enum CoordinatorCommand",
    )
}

fn managed_command_variant_names() -> BTreeSet<&'static str> {
    enum_variant_names_from_source_until(
        include_str!("../../../../canic-contracts/src/dto/wire/managed_command/mod.rs"),
        "pub enum CanisterCommand",
        "\n        }",
    )
}

fn store_command_variant_names() -> BTreeSet<&'static str> {
    enum_variant_names_from_source(
        include_str!("../../../../canic-contracts/src/dto/template/mod.rs"),
        "pub enum StoreCommand",
    )
}

fn emitted_endpoint_names(kind: EndpointKind) -> BTreeSet<String> {
    let mut directories = vec![
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../canic/src/macros/endpoints"),
    ];
    let mut names = BTreeSet::new();
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(directory).expect("endpoint source directory") {
            let entry = entry.expect("endpoint source entry");
            let path = entry.path();
            let file_kind = entry.file_type().expect("endpoint source file type");
            if file_kind.is_dir() {
                directories.push(path);
            } else if file_kind.is_file()
                && path.extension().is_some_and(|extension| extension == "rs")
            {
                let source = std::fs::read_to_string(path).expect("endpoint source");
                names.extend(
                    endpoint_names_from_source(&source, kind)
                        .into_iter()
                        .map(str::to_owned),
                );
            }
        }
    }
    assert!(
        !names.is_empty(),
        "endpoint source inventory must not be empty"
    );
    names
}

fn endpoint_names_from_source(source: &str, kind: EndpointKind) -> Vec<&str> {
    let attributes = match kind {
        EndpointKind::Query => ["#[$crate::canic_query", "#[$crate::canic_raw_query"],
        EndpointKind::Update => ["#[$crate::canic_update", "#[$crate::canic_raw_update"],
    };
    let lines = source.lines().collect::<Vec<_>>();
    let mut names = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if !attributes.iter().any(|attribute| line.contains(attribute)) {
            continue;
        }
        let Some(name) = lines
            .iter()
            .skip(index + 1)
            .find_map(|candidate| endpoint_name_from_fn_line(candidate))
        else {
            panic!("endpoint attribute without following function");
        };
        names.push(name);
    }
    names
}

fn endpoint_name_from_fn_line(line: &str) -> Option<&str> {
    let marker = "fn ";
    let start = line.find(marker)? + marker.len();
    let rest = &line[start..];
    let end = rest.find('(')?;
    Some(&rest[..end])
}
