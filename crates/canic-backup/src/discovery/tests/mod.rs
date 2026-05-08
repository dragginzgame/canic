use super::*;
use candid::Principal;
use serde_json::json;

const ROOT: Principal = Principal::from_slice(&[]);
const ROOT_TEXT: &str = "aaaaa-aa";
const APP_TEXT: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const WORKER_TEXT: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

// Build a deterministic non-root principal for discovery tests.
fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

// Ensure discovery projections produce valid manifest fleet sections.
#[test]
fn discovery_projects_to_valid_fleet_section() {
    let fleet = DiscoveredFleet {
        topology_records: vec![
            topology_record(ROOT, None, "root"),
            topology_record(p(2), Some(ROOT), "app"),
        ],
        members: vec![
            discovered_member("root", ROOT.to_string(), None),
            discovered_member("app", p(2).to_string(), Some(ROOT.to_string())),
        ],
    };

    let section = fleet
        .into_fleet_section()
        .expect("discovery should project");

    section.validate().expect("fleet section should validate");
    assert_eq!(section.discovery_topology_hash, section.topology_hash);
    assert_eq!(section.members.len(), 2);
}

// Ensure duplicate canisters are rejected before manifest projection.
#[test]
fn discovery_rejects_duplicate_canisters() {
    let fleet = DiscoveredFleet {
        topology_records: vec![topology_record(ROOT, None, "root")],
        members: vec![
            discovered_member("root", ROOT.to_string(), None),
            discovered_member("root", ROOT.to_string(), None),
        ],
    };

    let err = fleet
        .into_fleet_section()
        .expect_err("duplicate canisters should fail");

    assert!(matches!(err, DiscoveryError::DuplicateCanisterId(_)));
}

// Ensure discovery requires concrete member verification.
#[test]
fn discovery_requires_verification_checks() {
    let mut member = discovered_member("root", ROOT.to_string(), None);
    member.verification_checks.clear();
    let fleet = DiscoveredFleet {
        topology_records: vec![topology_record(ROOT, None, "root")],
        members: vec![member],
    };

    let err = fleet
        .into_fleet_section()
        .expect_err("missing verification should fail");

    assert!(matches!(err, DiscoveryError::MissingVerificationChecks(_)));
}

// Ensure registry parsing accepts the wrapped registry JSON shape.
#[test]
fn registry_entries_parse_wrapped_dfx_json() {
    let entries = parse_registry_entries(&registry_json()).expect("parse registry");

    assert_eq!(
        entries,
        vec![
            RegistryEntry {
                pid: ROOT_TEXT.to_string(),
                role: Some("root".to_string()),
                kind: Some("root".to_string()),
                parent_pid: None,
            },
            RegistryEntry {
                pid: APP_TEXT.to_string(),
                role: Some("app".to_string()),
                kind: Some("singleton".to_string()),
                parent_pid: Some(ROOT_TEXT.to_string()),
            },
            RegistryEntry {
                pid: WORKER_TEXT.to_string(),
                role: Some("worker".to_string()),
                kind: Some("replica".to_string()),
                parent_pid: Some(APP_TEXT.to_string()),
            },
        ]
    );
}

// Ensure non-recursive target resolution includes only direct children.
#[test]
fn registry_targets_include_direct_children() {
    let entries = parse_registry_entries(&registry_json()).expect("parse registry");
    let targets = targets_from_registry(&entries, ROOT_TEXT, false).expect("resolve targets");
    let ids = targets
        .iter()
        .map(|target| target.canister_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, vec![ROOT_TEXT, APP_TEXT]);
}

// Ensure recursive target resolution walks the full subtree.
#[test]
fn registry_targets_include_recursive_children() {
    let entries = parse_registry_entries(&registry_json()).expect("parse registry");
    let targets = targets_from_registry(&entries, ROOT_TEXT, true).expect("resolve targets");
    let ids = targets
        .iter()
        .map(|target| target.canister_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, vec![ROOT_TEXT, APP_TEXT, WORKER_TEXT]);
}

// Build one topology record for tests.
fn topology_record(pid: Principal, parent_pid: Option<Principal>, role: &str) -> TopologyRecord {
    TopologyRecord {
        pid,
        parent_pid,
        role: role.to_string(),
        module_hash: None,
    }
}

// Build one discovered member for tests.
fn discovered_member(
    role: &str,
    canister_id: String,
    parent_canister_id: Option<String>,
) -> DiscoveredMember {
    DiscoveredMember {
        role: role.to_string(),
        canister_id,
        parent_canister_id,
        subnet_canister_id: None,
        controller_hint: Some(ROOT.to_string()),
        identity_mode: IdentityMode::Fixed,
        verification_checks: vec![VerificationCheck {
            kind: "status".to_string(),
            roles: Vec::new(),
        }],
        snapshot_plan: SnapshotPlan {
            snapshot_id: format!("snap-{role}"),
            module_hash: Some(HASH.to_string()),
            wasm_hash: Some(HASH.to_string()),
            code_version: Some("v0.30.0".to_string()),
            artifact_path: format!("artifacts/{role}"),
            checksum_algorithm: "sha256".to_string(),
            checksum: Some(HASH.to_string()),
        },
    }
}

// Build representative subnet registry JSON.
fn registry_json() -> String {
    json!({
        "Ok": [
            {
                "pid": ROOT_TEXT,
                "role": "root",
                "record": {
                    "pid": ROOT_TEXT,
                    "role": "root",
                    "kind": "root",
                    "parent_pid": null
                }
            },
            {
                "pid": APP_TEXT,
                "role": "app",
                "record": {
                    "pid": APP_TEXT,
                    "role": "app",
                    "kind": "singleton",
                    "parent_pid": ROOT_TEXT
                }
            },
            {
                "pid": WORKER_TEXT,
                "role": "worker",
                "record": {
                    "pid": WORKER_TEXT,
                    "role": "worker",
                    "kind": "replica",
                    "parent_pid": [APP_TEXT]
                }
            }
        ]
    })
    .to_string()
}
