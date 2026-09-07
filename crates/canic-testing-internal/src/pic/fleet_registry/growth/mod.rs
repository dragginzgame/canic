//! Module: pic::fleet_registry::growth
//!
//! Responsibility: generate retained paid-growth authority for the existing PocketIC journey.
//! Does not own: funding effects, runtime recovery or application-specific topology.
//! Boundary: publishes synthetic policy/catalog inputs and calls the production generator.

mod catalog;
mod source;

use canic_host::{
    fleet_ensure::{FleetGenerateRequest, generate_desired_fleet, model::DesiredFleet},
    icp::LocalReplicaTarget,
};
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt as _, path::Path};

pub(super) fn generate(
    workspace: &Path,
    config: &Path,
    icp: &Path,
    replica: &LocalReplicaTarget,
    retained: &DesiredFleet,
) -> DesiredFleet {
    let bootstrap = retained.bootstrap.as_ref().unwrap();
    let root = &bootstrap.roots[0];
    let principal = |name: &str| {
        retained
            .canisters
            .iter()
            .find(|canister| canister.name == name)
            .unwrap()
            .principal
            .as_ref()
            .unwrap()
            .clone()
    };
    let seed = workspace.join("growth-seed.toml");
    let source = workspace.join("growth-policy.toml");
    let imports = root
        .canister_pool_imports
        .iter()
        .map(|name| principal(name))
        .collect::<Vec<_>>();
    let document = json!({
        "schema_version": 1,
        "fleet_id": bootstrap.fleet_id,
        "fresh_estate": false,
        "coordinator": principal(&bootstrap.coordinator),
        "cycles_ledger": retained.cycles_ledger,
        "management_creation_fee_cycles": "500B",
        "roots": [{
            "placement_subnet": root.placement_subnet.to_string(),
            "root": principal(&root.root),
            "store": principal(&root.store),
            "pool_imports": imports,
        }],
    });
    fs::write(&seed, toml::to_string(&document).unwrap()).unwrap();
    fs::write(
        &source,
        toml::to_string(&source::document(retained)).unwrap(),
    )
    .unwrap();
    catalog::retain(workspace, retained);
    // Only transport is local: generation validates the sealed IC build, the
    // current Root authority and the synthetic Registry-backed placement evidence.
    let wrapper = workspace.join("growth-generator-icp");
    fs::write(
        &wrapper,
        format!(
            r#"#!/bin/bash
set -euo pipefail
case " $* " in
  *" canister "*|*" cycles "*)
    unset ICP_ENVIRONMENT
    args=()
    while (( $# )); do
      case "$1" in
        -e|--environment) shift 2 ;;
        *) args+=("$1"); shift ;;
      esac
    done
    exec '{}' "${{args[@]}}" -n '{}' -k '{}' ;;
  *) exec '{}' "$@" ;;
esac
"#,
            icp.display(),
            replica.url,
            replica.root_key,
            icp.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    let generated = generate_desired_fleet(&FleetGenerateRequest {
        app_config: config,
        environment: "ic",
        fleet: &retained.fleet,
        icp_executable: wrapper.to_str().unwrap(),
        release_build_id: bootstrap.release_build_id,
        root: workspace,
        seed: &seed,
        source: &source,
    })
    .expect("generate retained paid growth before any funding effect");
    assert_eq!(generated.observed_canisters, retained.canisters.len());
    assert_eq!(generated.desired.management_creation_fee_cycles, "500B");
    assert_eq!(generated.desired.ledger_fee_cycles, "0.1B");
    assert_eq!(
        generated.desired.bootstrap.as_ref().unwrap().fleet_id,
        bootstrap.fleet_id
    );
    generated.desired
}
