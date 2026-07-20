use super::*;
use crate::{CliError, cli_error_exit_code};
use canic_core::{
    ids::CanisterRole,
    role_contract::{
        AllocationOwner, CanicFeatureKey, ResolvedRoleContract, ResolvedStateAllocation,
        RoleCapabilityKey, SelectionProvenance, StateAllocationKey,
        allocation::allocation_definition,
    },
};
use canic_host::role_contract::materialize_state_manifest;
use std::{collections::BTreeSet, path::PathBuf};

fn test_state_manifest(role: Option<&str>) -> StateManifest {
    let contracts = if matches!(role, None | Some("root")) {
        vec![root_contract()]
    } else {
        Vec::new()
    };
    materialize_state_manifest(&contracts).expect("test state manifest")
}

fn build_state_audit_report(role: Option<&str>) -> StateAuditReport {
    let resolution = StateManifestResolution::Resolved {
        manifest: test_state_manifest(role),
        contracts: Vec::new(),
    };
    canic_host::state_manifest::build_state_audit_report(&resolution, role)
}

fn root_contract() -> ResolvedRoleContract {
    let keys = [
        StateAllocationKey::CoreRuntimeTopology,
        StateAllocationKey::CoreRootAppRegistry,
        StateAllocationKey::CoreRuntimeEnvironment,
        StateAllocationKey::CoreAuthState,
        StateAllocationKey::CoreReplayReceipts,
        StateAllocationKey::CoreRuntimeObservability,
        StateAllocationKey::CoreRuntimeIntent,
        StateAllocationKey::CanisterPool,
        StateAllocationKey::TemplateManifests,
        StateAllocationKey::TemplateChunkSets,
        StateAllocationKey::TemplateChunkRefs,
        StateAllocationKey::TemplateChunkPayloads,
        StateAllocationKey::ControlPlaneSubnetState,
    ];
    let allocations = keys
        .into_iter()
        .map(|key| {
            let definition = allocation_definition(key).expect("allocation definition");
            ResolvedStateAllocation {
                key,
                owner: definition.owner,
                memory_ids: definition.memory_ids.to_vec(),
                selected_by: BTreeSet::from([
                    if definition.owner == AllocationOwner::CanicControlPlane {
                        SelectionProvenance::EffectiveFeature(CanicFeatureKey::ControlPlane)
                    } else {
                        SelectionProvenance::Capability(RoleCapabilityKey::Root)
                    },
                ]),
            }
        })
        .collect();
    ResolvedRoleContract {
        role: CanisterRole::ROOT,
        built_in: None,
        capabilities: BTreeSet::new(),
        required_features: BTreeSet::new(),
        effective_features: BTreeSet::new(),
        allocations,
    }
}

#[test]
fn state_command_exposes_audit_and_manifest_subcommands() {
    let parsed = parse_required_subcommand(
        state_command(),
        [OsString::from(AUDIT_COMMAND), OsString::from("--json")],
    )
    .expect("parse state audit");

    assert_eq!(parsed.0, AUDIT_COMMAND);
    assert_eq!(parsed.1, vec![OsString::from("--json")]);

    let help = usage();
    assert!(help.contains("canic state audit"));
    assert!(help.contains("canic state manifest"));
    assert!(help.contains("diagnostic-only"));
    assert!(help.contains("do not read stable"));
    assert!(help.contains("write generated files"));
    assert!(help.contains("mutate canisters"));
}

#[test]
fn parses_supported_audit_options() {
    let options = StateOptions::parse_audit([
        OsString::from("--role"),
        OsString::from("root"),
        OsString::from("--json"),
    ])
    .expect("parse audit options");

    assert_eq!(options.role.as_deref(), Some("root"));
    assert!(options.json);
}

#[test]
fn parses_supported_manifest_options() {
    let options = StateOptions::parse_manifest([
        OsString::from("--role"),
        OsString::from("root"),
        OsString::from("--json"),
    ])
    .expect("parse manifest options");

    assert_eq!(options.role.as_deref(), Some("root"));
    assert!(options.json);
}

#[test]
fn audit_json_uses_schema_version_two() {
    let report = build_state_audit_report(Some("root"));
    let json = serde_json::to_value(&report).expect("state audit report serializes");

    assert_eq!(json["schema_version"], 2);
    assert_eq!(json["command"], "canic state audit");
    assert_eq!(json["scope"], "role");
    assert_eq!(json["role"], "root");
    assert_eq!(json["status"], "warn");
    assert!(
        json["checks"]
            .as_array()
            .expect("checks")
            .iter()
            .any(|check| check["code"] == "reserved_memory_id_declared")
    );
    assert!(
        json["checks"]
            .as_array()
            .expect("checks")
            .iter()
            .any(|check| check["code"] == "reserved_export_import_ok")
    );
}

#[test]
fn manifest_json_is_manifest_directly() {
    let manifest = test_state_manifest(Some("root"));
    let json = serde_json::to_value(&manifest).expect("state manifest serializes");

    assert_eq!(json["schema_version"], 2);
    assert!(json.get("command").is_none());
    assert_eq!(json["roles"][0]["canister_role"], "root");
    assert!(
        json["roles"][0]["state"]
            .as_array()
            .expect("state domains")
            .iter()
            .any(|entry| entry["domain"] == "runtime_log")
    );
}

#[test]
fn text_renderers_include_stable_fields() {
    let report = build_state_audit_report(Some("root"));
    let audit = render_audit_text(&report);
    let manifest = render_manifest_text(&test_state_manifest(Some("root")));

    assert!(audit.contains("schema_version: 2"));
    assert!(audit.contains("scope: role"));
    assert!(audit.contains("memory_id [warn] reserved_memory_id_declared"));
    assert!(audit.contains("source: state_manifest"));
    assert!(manifest.contains("canic state manifest"));
    assert!(manifest.contains("migration_policy: new_domain"));
    assert!(manifest.contains("template_manifests"));
    assert!(manifest.contains("reserved_memory"));
    assert!(manifest.contains("runtime_log"));
}

#[test]
fn failing_state_audit_uses_exit_code_one() {
    let err = run_audit(vec![OsString::from("--role"), OsString::from("missing")])
        .expect_err("missing role fails audit");

    assert!(matches!(err, StateCommandError::AuditFailed));
    assert_eq!(
        cli_error_exit_code(&CliError::State(err)),
        i32::from(StateCommandError::AuditFailed.exit_code())
    );
}

#[test]
fn state_command_preserves_project_discovery_causes() {
    let root_error = StateCommandError::from(IcpConfigError::NoIcpRoot {
        start: PathBuf::from("/missing/project"),
    });
    assert!(matches!(
        root_error,
        StateCommandError::IcpRoot(IcpConfigError::NoIcpRoot { .. })
    ));

    let discovery_error = StateCommandError::from(ConfigDiscoveryError::DuplicateFleet {
        fleet: "duplicate".to_string(),
        configs: "first/canic.toml, second/canic.toml".to_string(),
    });
    assert!(matches!(
        discovery_error,
        StateCommandError::ConfigDiscovery(ConfigDiscoveryError::DuplicateFleet { .. })
    ));
}
